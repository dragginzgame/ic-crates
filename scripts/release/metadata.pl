#!/usr/bin/env perl
use strict;
use warnings;
use Digest::SHA qw(sha256_hex);
use JSON::PP;
use File::Temp qw(tempfile);

# Application-owned release receipt; Shared Tooling owns Git delivery/recovery.
my @files = qw(Cargo.toml Cargo.lock CHANGELOG.md);
my @release_files = (@files, 'docs/release.json');
my $json = JSON::PP->new->canonical->pretty;
my ($operation, @args) = @ARGV;

sub read_file {
    my ($path) = @_;
    -f $path && !-l $path or die "expected regular release input: $path\n";
    open my $input, '<:raw', $path or die "$path: $!\n";
    local $/;
    my $value = <$input>;
    close $input or die "$path: $!\n";
    return $value;
}

sub command_output {
    open my $pipe, '-|', @_ or die "cannot start observation\n";
    local $/;
    my $value = <$pipe> // '';
    close $pipe or die "observation failed\n";
    return $value;
}

sub git_output { return command_output('git', @_); }


sub identity {
    my ($source, $version, $date) = @_;
    $source =~ /\A(?:[0-9a-f]{40}|[0-9a-f]{64})\z/ or die "invalid source identity\n";
    $version =~ /\A(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\z/
        or die "invalid release version\n";
    $date =~ /\A[0-9]{4}-[0-9]{2}-[0-9]{2}\z/ or die "invalid release date\n";
}

sub decode_receipt {
    my ($bytes) = @_;
    length($bytes) <= 8192 or die "receipt exceeds byte limit\n";
    my $receipt = $json->decode($bytes);
    ref($receipt) eq 'HASH' && keys(%$receipt) == 5 or die "invalid receipt schema\n";
    for (qw(format source version date files)) { exists $receipt->{$_} or die "missing receipt field\n"; }
    !ref($receipt->{format}) && $receipt->{format} eq '1' or die "invalid receipt format\n";
    identity(@{$receipt}{qw(source version date)});
    ref($receipt->{files}) eq 'HASH' && keys(%{$receipt->{files}}) == @files
        or die "invalid receipt input roster\n";
    for (@files) {
        ($receipt->{files}{$_} // '') =~ /\A[0-9a-f]{64}\z/ or die "invalid input digest\n";
    }
    return $receipt;
}

sub check_receipt {
    my ($receipt, $source, $version, $date, $read) = @_;
    identity($source, $version, $date);
    $receipt->{source} eq $source && $receipt->{version} eq $version && $receipt->{date} eq $date
        or die "release receipt identity differs\n";
    for (@files) { sha256_hex($read->($_)) eq $receipt->{files}{$_} or die "release input differs: $_\n"; }
}

sub check_paths {
    my ($bytes) = @_;
    my @paths = split /\0/, $bytes;
    my %expected = map { $_ => 1 } @release_files;
    @paths == @release_files or die "release index must contain exactly its metadata files\n";
    for (@paths) { delete $expected{$_} or die "unexpected release path: $_\n"; }
    !keys(%expected) or die "incomplete release metadata\n";
}

if (($operation // '') eq 'receipt') {
    @args == 3 or die "receipt expects source version date\n";
    identity(@args);
    my %hashes = map { $_ => sha256_hex(read_file($_)) } @files;
    my $receipt = {format => 1, source => $args[0], version => $args[1], date => $args[2], files => \%hashes};
    -d 'docs' && !-l 'docs' or die "expected regular docs directory\n";
    -l 'docs/release.json' and die "receipt must not be a symlink\n";
    -d 'target' && !-l 'target' or die "expected regular target directory\n";
    my ($out, $path) = tempfile('release-receipt.XXXXXX', DIR => 'target', UNLINK => 0);
    print {$out} $json->encode($receipt) or die "receipt write failed\n";
    close $out or die "receipt close failed\n";
    rename $path, 'docs/release.json' or die "receipt publication failed: $!\n";
} elsif (($operation // '') eq 'check') {
    @args == 3 or die "check expects source version date\n";
    check_receipt(decode_receipt(read_file('docs/release.json')), @args, \&read_file);
} elsif (($operation // '') eq 'index-check') {
    @args == 0 or die "index-check expects no arguments\n";
    check_paths(git_output('diff', '--cached', '--name-only', '-z', '--'));
} elsif (($operation // '') eq 'check-commit') {
    @args == 4 or die "check-commit expects commit source version date\n";
    my ($commit, $source, $version, $date) = @args;
    identity($commit, $version, $date);
    my $parent = git_output('log', '-1', '--format=%P', $commit);
    $parent =~ s/\n\z//;
    $parent eq $source or die "release parent differs from validated source\n";
    check_paths(git_output('diff', '--name-only', '-z', $source, $commit, '--'));
    my $receipt = decode_receipt(git_output('show', "$commit:docs/release.json"));
    check_receipt($receipt, $source, $version, $date, sub { git_output('show', "$commit:$_[0]") });
} else {
    die "usage: metadata.pl receipt|check|index-check|check-commit ...\n";
}

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

sub set_version {
    my ($previous, $candidate) = @_;
    identity('0' x 40, $previous, '2000-01-01');
    identity('0' x 40, $candidate, '2000-01-01');
    $previous ne $candidate or die "candidate must differ from previous\n";
    my $model = $json->decode(command_output('yq', '-p', 'toml', '-o', 'json', '.', 'Cargo.toml'));
    my $package_version = $model->{workspace}{package}{version};
    my $dependency_version = $model->{workspace}{dependencies}{'ic-crates-protocol'}{version};
    for ($package_version, $dependency_version) {
        defined($_) && ($_ eq $previous || $_ eq $candidate) or die "manifest differs from release intent\n";
    }
    my $text = read_file('Cargo.toml');
    my ($section, $package_changes, $dependency_changes) = ('', 0, 0);
    my @lines = split /(?<=\n)/, $text;
    for my $line (@lines) {
        if ($line =~ /^\[([^\]\r\n]+)\][ \t]*(?:\#.*)?\n?\z/) { $section = $1; }
        if ($section eq 'workspace.package') {
            $package_changes += $line =~ s/^(version[ \t]*=[ \t]*)"\Q$package_version\E"/$1"$candidate"/;
        } elsif ($section eq 'workspace.dependencies' && $line =~ /^ic-crates-protocol[ \t]*=/) {
            $dependency_changes += $line =~ s/\bversion[ \t]*=[ \t]*"\Q$dependency_version\E"/version = "$candidate"/;
        } elsif ($section eq 'workspace.dependencies.ic-crates-protocol') {
            $dependency_changes += $line =~ s/^(version[ \t]*=[ \t]*)"\Q$dependency_version\E"/$1"$candidate"/;
        }
    }
    $package_changes == 1 && $dependency_changes == 1 or die "cannot locate unique version assignments\n";
    -d 'target' && !-l 'target' or die "expected regular target directory\n";
    my ($out, $path) = tempfile('release-manifest.XXXXXX', DIR => 'target', UNLINK => 0);
    print {$out} join('', @lines) or die "manifest write failed\n";
    close $out or die "manifest close failed\n";
    chmod((stat('Cargo.toml'))[2] & 0777, $path) or die "manifest mode failed\n";
    rename $path, 'Cargo.toml' or die "manifest publication failed\n";
}

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

if (($operation // '') eq 'set-version') {
    @args == 2 or die "set-version expects previous candidate\n";
    set_version(@args);
} elsif (($operation // '') eq 'receipt') {
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
    die "usage: metadata.pl set-version|receipt|check|index-check|check-commit ...\n";
}

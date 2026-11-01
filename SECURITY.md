# Security

Please report a vulnerability privately rather than in a public issue: through
[a private advisory](https://github.com/sachesi/modpdb/security/advisories/new) on GitHub,
or by mail to sachesi <xsachesi@pm.me>. Say what you found, how to reproduce it and which
version you ran; a fix is worked out with you before anything is published.

There are no releases yet, so fixes go to the `main` branch.

## What counts

modpdb keeps a list of kernel modules, and two of its commands run as root: `sudo modpdb
recall` and `sudo modpdb rebuild`. Both read that list and `modpdb.conf` from the home of
the user who ran sudo, so whatever is in those files reaches a root process. The parts
where a mistake matters most:

- Names from the database, which are passed to `modprobe`. A line in the database that
  passes an option to `modprobe`, or starts anything but `modprobe`, is a vulnerability.
- What the root commands write: the database and its backup, in the configured `DBPATH`. A
  way to make a root run create or overwrite any other file is a vulnerability.
- Which user's files a root run takes. A way to make it read another user's configuration
  by anything other than running sudo as that user is a vulnerability.

Loading a module that the database names is what `recall` and `rebuild` do. Someone who
can already run them through sudo, and edits the database to load a module installed on
the system, gains nothing they did not have, so that is not a vulnerability. A flaw in
`modprobe` or a module belongs with that project. A module missing from the list is a bug;
please file it as an ordinary issue.

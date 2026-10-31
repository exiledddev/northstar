# Security

## Reporting a problem

Please **do not open a public issue** for a security problem.

Report it privately through GitHub: the repository's **Security** tab →
**Report a vulnerability**. If that is not available to you, email
**hello@nativecompute.com** with "Blackline security" in the subject.

Tell us what you found, how to reproduce it, and what an attacker could do
with it. We will acknowledge the report within a few days, keep you posted
while we fix it, and credit you in the release notes if you would like.

## What is in scope

- The desktop app: reading a crafted script or import file (Fountain, Final
  Draft, markdown) that crashes it, hangs it, or makes it write somewhere it
  should not.
- The browser build and its server, in
  [exiledddev/northstarWeb](https://github.com/exiledddev/northstarWeb):
  sign-in, sessions, who may read or write which script.

## Supported versions

Security fixes go into the latest release only.

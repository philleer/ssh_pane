# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive reports.

Email the maintainer: **phillee2016@163.com**

Include:

- Affected version / commit
- Impact (e.g. credential leak, remote code risk, MITM)
- Steps to reproduce or a proof-of-concept if available
- Whether you plan a public disclosure date

You should receive an acknowledgement within a few days. After a fix is available (or a clear decision is made), we can credit you in the release notes if you want.

## Current known gaps (v0.1)

These are documented limitations, not secret bugs:

- SSH host keys are not checked against `known_hosts`
- Passphrase-protected private keys are unsupported
- Passwords are held in process memory during the session and are not written to disk

# Security Policy

## Supported Versions

Currently, only the `main` branch is actively supported with security updates.

## Reporting a Vulnerability

If you discover a security vulnerability within this project, please send an email to the project maintainers. All security vulnerabilities will be promptly addressed.

## Addressing Security Alerts

Our CI pipeline includes automated security scanning for dependencies:
<<<<<<< HEAD
1. **GitHub Actions Workflow**: Every PR is scanned using `npm audit --audit-level=high`. The build will fail if any high-severity vulnerabilities are found.
2. **Dependabot**: We use Dependabot to automatically check for dependency updates on a weekly basis.
=======
1. **GitHub Actions Workflow**: Every PR is scanned using `npm audit --audit-level=high` (for JS/TS packages) and standard security pipelines for other languages. The build will fail if any high-severity vulnerabilities are found.
2. **Dependabot**: We use Dependabot to automatically check for dependency updates on a weekly schedule across all ecosystem (npm, cargo).
>>>>>>> ab330db (OAuth-Integration-and-Automated-Security-Scanning)

### Process for Patching Vulnerabilities

1. **Review the Alert**: When Dependabot opens a PR or a CI pipeline fails due to an `npm audit` alert, review the identified vulnerability and the package involved.
2. **Verify the Fix**:
   - For Dependabot PRs, verify that the PR fixes the vulnerability by checking the release notes of the updated package. Ensure the CI passes.
   - For `npm audit` failures, run `npm audit fix` locally to apply non-breaking updates. If manual intervention is required, update the dependency in `package.json` to the secure version and test the application to ensure nothing is broken.
3. **Merge the Fix**: Once the fix is verified and tests pass, approve and merge the PR. 
4. **Emergency Patches**: In the event of a critical zero-day vulnerability, maintainers will bypass the weekly schedule and immediately create a hotfix PR to update the affected dependency.

By following this process, we ensure that no known high-severity vulnerabilities are introduced into the codebase and that any existing issues are resolved promptly.

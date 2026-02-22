# Contributing to SysOps

Thanks for your interest in contributing!

## Development Setup

1. Fork the repository
2. Clone your fork
3. Create a feature branch: `git checkout -b feat/my-feature`
4. Make your changes
5. Run tests and linting
6. Commit with a descriptive message
7. Push and create a Pull Request

## Code Style

### Rust (Agent & Server)
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Write tests for new functionality

### TypeScript (Console)
- Follow existing code conventions
- Use TypeScript strict mode
- Test UI changes across different screen sizes

## Commit Messages

Use conventional commits:
- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `refactor:` code restructuring
- `chore:` maintenance

## Related Repositories

- [sysops-agent](https://github.com/ecmoce/sysops-agent) — Monitoring agent
- [sysops-server](https://github.com/ecmoce/sysops-server) — Central server
- [sysops-console](https://github.com/ecmoce/sysops-console) — Web UI

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

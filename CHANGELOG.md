# v0.1.0
### Styling issues:
- App help page dialog no longer clips with top app bar
- Removed small artifact caused by styling of the actions row in the main app view

### Enhancements:
- Add UI loading indications for all operations
- All blocking operations are now happening in blocking threads. This leads to a responsive UI even during demanding workloads.
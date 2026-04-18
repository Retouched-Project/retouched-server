<p align="left">
  <img src="assets/retouched_logo_text_server.svg" alt="Retouched Server Logo" width="30%"/>
</p>

> [!NOTE]
> **This is not an officially supported Ntrome Ltd. or Infrared5 Inc. product.**

# Retouched Server
A reverse engineered implementation of the Brass Monkey server in Rust.

## Platform Support

|         | x86 | x86_64 | arm32 | arm64 |
|---------|:---:|:------:|:-----:|:-----:|
| Windows | ⚠️  | ✅     | N/A   | ✅    |
| Linux   | ⚠️  | ✅     | ⚠️    | ✅    |
| macOS   | N/A | ✅     | N/A   | ✅    |

✅ GUI &nbsp; ⚠️ CLI only

## Installation
Windows:
- Download and extract the zip. An installer is planned.
- You might have to press "Run anyway" from SmartScreen.

Linux:
- The appimage is recommended. Glibc 2.39+ is required (musl is unsupported -> use a compat layer).
- The tarball might work on distros with Qt older than 6.10.3 (Qt 6.3 or higher required) but this hasn't been tested.

macOS:
- Open the dmg and drag the icon into the Applications folder.
- You will have to open the Apps folder with Finder separately for now.
- You will have to allow the app to be run from Settings.

## TODO
- [X] Make sure all targets can be built from GitHub actions and they work. (v1.0.0 requirement)
- [ ] Add an about page. (v1.0.1)
- [ ] Improve Retouched Web update UX. (v1.0.2)
- [ ] Update checker (v1.0.3)
- [ ] Switch from polling to pushing updates to the Qt GUI. (v1.1.0)
- [ ] AUR packaging (x86_64 only)
- [ ] Binary signing
- [ ] Windows installer
- [ ] Better macOS dmg with drag and drop and background

## License

This project is licensed under the AGPL-3.0 License.  
See the [LICENSE](LICENSE) file for details.

Images in this repository are licensed under the Creative Commons Attribution 4.0 International License.  
See the [LICENSE-IMAGES.md](LICENSE-IMAGES.md) file for details.

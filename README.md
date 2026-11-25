# AnyRoblox (Any Games To Roblox)

Our goal is to bring 3D models from any game into Roblox. Currently supported: Arma 3 and FiveM for OBJ export, Arma Reforger for Workshop page inspection via CLI.

AnyRoblox converts FiveM and Arma 3 models to Wavefront OBJ with MTL materials and diffuse PNG textures. The desktop app includes a 3D viewer. Local conversion works offline. The Arma 3 command-line tool can also download Workshop items when Steam publishes a direct file URL.

## Windows desktop app

Download [AnyRoblox-win-x64.zip](https://github.com/niskil377-dotcom/AnyRoblox/releases/latest/download/AnyRoblox-win-x64.zip), extract the whole folder and run `AnyRoblox.exe`. Keep the `tools` folder next to the executable. Choose **FiveM → OBJ** or **Arma 3 → OBJ**. Use **Switch workspace** to return to the selection screen.

### FiveM

Select a GTA V Legacy YDR, YDD or YFT file, or choose the `stream` folder of a local resource. Click **Convert to OBJ** and choose an output location. The app opens the exported models in the viewer and saves a batch report in the output folder.

The exporter reads the first available visual level of detail (LOD), including triangles, normals, UV coordinates and the material assigned to each geometry. A YDD can produce several OBJ files. A YFT exports its primary drawable. Animated skeletons, damage fragments, cloth simulation and collisions are outside the export scope. The exporter can write the `DiffuseSampler` texture to PNG from the model itself or from a neighboring YTD with the same base name. Textures shared across resources need additional resolution. Protected files and GTA V Enhanced formats are unsupported.

The desktop build caches CodeWalker.Core 1.0.3 and SharpDX 4.2.0 from NuGet, verifies their SHA-256 hashes and places the libraries in the release's `tools` folder. Validation used Legacy YDR and YFT binaries from the Sollumz test suite and a synthetic YDD built from the YDR. The exported models opened in the viewer with 12, 24 and 12 triangles respectively. [CodeWalker](https://github.com/dexyfex/CodeWalker) · [Sollumz test assets](https://github.com/Sollumz/Sollumz/tree/main/tests/assets/gen8)

### Arma 3

1. Subscribe to the mod in the Arma 3 Steam Workshop and wait for Steam to finish downloading it.
2. Open AnyRoblox, or click **Refresh** if the app is already open. Select the mod under **Installed mods**.
3. Click **Convert to OBJ** and choose an output location outside the mod folder. The converted models appear in the viewport. Click **Open output folder** to access the exports and reports.

The app searches Steam libraries listed in `libraryfolders.vdf`. It checks `steamapps/workshop/content/107410`, the game's `!Workshop` and `mods` folders, and the `mods` folder beside the executable. **Library…** adds a custom mod folder and remembers it for the next launch. **Folder…** and **PBO / ZIP…** let you select an individual source before conversion.

### Viewing models

The 3D viewport is on the left. Select a model in the **Outliner** on the right. **Conversion Properties** contains source selection and export controls. Drag the dividers to resize the panels. Toggle **Grid** to show the ground plane or **Solid** to inspect geometry with a neutral material. Use **Open OBJ…** to inspect an existing export.

| Control | Action |
|---|---|
| Mouse wheel | Zoom |
| Middle mouse button + drag | Orbit |
| Shift + middle mouse button + drag | Pan |
| F | Frame the model |

### Building the desktop app

The Windows x64 release is a folder: `AnyRoblox.exe` (the WPF viewer) and a `tools` folder holding the Rust CLI, P3DDebin, HEMTT and the FiveM exporter as separate files. The app runs them from there and never unpacks executables at run time. It uses .NET Framework 4.8. To rebuild it, place the conversion tools in `cache/outils` and run `./desktop/build.ps1` in PowerShell. The script writes `dist/AnyRoblox/` and `dist/AnyRoblox-win-x64.zip`. Pass `-SigningThumbprint <thumbprint>` to sign the project's executables with a code-signing certificate.

## Arma 3 command-line tool

Install stable Rust, Cargo and the linker for your target, then run the following commands from the project root.

```sh
cargo test
cargo build --release
cargo install --path . --locked
arma2obj --workshop "https://steamcommunity.com/sharedfiles/filedetails/?id=3842854567" --output ./output/workshop
arma2obj --workshop 3842854567 --output ./output/by-id
arma2obj --input ./my-mod --output ./output/local
```

The Cargo install step is optional. The built binary is also available in `target/release`. Each run takes an empty or new output directory outside the input. `--input` accepts a directory, ZIP archive or PBO file. Archives are extracted into temporary directories, and external tools receive working copies.

To reproduce the ACE3 validation on Windows, place the sample PBO and tools at the paths shown below. The cache, validation outputs and external executables are excluded from Git.

```powershell
.\target\release\arma2obj.exe --input .\cache\validation\ace3-3.21.3\ace_dogtags.pbo --debinarizer .\cache\outils\P3DDebin.exe --paa-tool .\cache\outils\hemtt.exe --output .\sortie\test
```

Set `ARMA2OBJ_DEBINARIZER` and `ARMA2OBJ_PAA_TOOL` to reuse the same tool paths across runs. Explicit CLI options take precedence. On the validation machine, the Windows GNU toolchain used Cargo and dlltool from the following session PATH.

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$(Resolve-Path .\cache\outils\rust-gnu);$env:Path"
cargo build --release
```

## Workshop downloads

The CLI normalizes the item ID, sends an anonymous POST and checks the returned ID, result code, public visibility, ban status and Arma 3 application ID (107410). The report records the title and the creator's SteamID. Successful downloads go into `./cache/<id>/`. Use `--cache` to change the location. When Steam provides an update timestamp, the cache distinguishes revisions. The CLI checks public access again before reusing a cached file. It identifies ZIP files by their signature and PBO files by validating their entry table. For 7z, RAR and gzip, it identifies the format and asks for local extraction. [Steam API](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage#GetPublishedFileDetails)

**November 24, 2026 test.** For item `3842854567`, Steam returned HTTP 200, the title “RZ - M60 REVIVAL”, creator `76561198416561244`, public visibility and an empty `file_url`. No archive was received, so its format, models and textures could not be verified. To convert that item, supply a local archive or installed mod folder. The CLI also reports HTTP refusal, private items and network failures. If the network blocks Steam, supply the archive manually. [Tested Workshop item](https://steamcommunity.com/sharedfiles/filedetails/?id=3842854567)

**Arma Reforger.** The “Sample Mod - New Car” page displayed the author, description, license, version, size and item ID. Visible links included images and the changelog. The review found no direct link to the mod archive. For a Reforger URL, the CLI reads the page title and reports this limitation. Local input supports compatible Arma 3 assets. Native Enfusion formats are outside the scope of this version. [Reviewed Reforger item](https://reforger.armaplatform.com/workshop/5614E482BF83E310-SampleMod-NewCar)

## Arma 3 formats and tools

| Stage | Support and conventions |
|---|---|
| Archives | Stored and Deflate ZIP entries. PBOs with contiguous entries, Vers properties, virtual prefixes, and raw or Cprs/LZSS entries. The reader verifies the Cprs additive checksum and recognizes the optional 21-byte SHA trailer by its structure. Cryptographic signatures and bisign files are outside its checks. |
| Read limits | 512 MiB per archive or model read into memory, 100,000 entries per archive, 2 GiB of decompressed files per ZIP and 2 GiB of PBO extraction per batch. Names must be UTF-8 or ASCII. Proprietary or obfuscated packages may be rejected. |
| ODOL | Reads the signature and little-endian version. The CLI accepts v75 by default. Use `--odol-version 73,74,75` to replace the version list for an external tool with known capabilities. The MLOD reader validates the debinarizer's output. |
| MLOD | Version 257 with P3DM 28/256 blocks, points, normals, triangles and quads, four corner slots per face, base UVs, per-face texture and RVMAT, TAGG records and LOD resolution. The reader selects the lowest resolution greater than or equal to 0 and less than 1000. It reports a failure if no visual LOD exists. |
| OBJ and MTL | One-based indices, per-corner normals and UVs, and groups by texture/RVMAT pair. OBJ uses Y up. The exporter negates the third position coordinate, adjusts normal signs and converts V to 1−V. Transparent PNGs also supply the opacity map. |
| Textures | Resolves references across all PBOs in the batch using the virtual prefix, then relative to the model. Lookup is ASCII case-insensitive. For extracted directories, `$PBOPREFIX$` and `$PREFIX$` markers restore virtual paths. A PNG with the same base name can supply the texture directly. If conversion is unavailable, the material keeps its diffuse color and the report records a partial export. |
| Inventory | LOD resolutions, HitPoints LODs, proxies, named selections and Animation tag times. For nearby or parent model.cfg files, the inventory lists skeleton and animation classes declared in text. These declarations may cover several models. Faces belonging to identified proxies are excluded. |
| Outside the export scope | Technical LODs, collisions, bone weights and hierarchies, animations, proxy target models, RVMAT shaders and stages, procedural textures and secondary UVs. The reader inventories model.cfg declarations without preprocessing or inheritance resolution. Empty lists reflect what the reader observed. |

The PBO reader follows the documented fields and Bohemia's LZSS variant. The MLOD layout was cross-checked against the Arma 3 Object Builder reader and verified on an actual MLOD file. [PBO format](https://community.bistudio.com/wiki/PBO) · [LZSS](https://community.bistudio.com/wiki/Compressed_LZSS_File_Format) · [MLOD format](https://community.bistudio.com/wiki/P3D_File_Format_-_MLOD) · [Reference reader](https://github.com/MrClock8163/Arma3ObjectBuilder/blob/master/Arma3ObjectBuilder/io/data_p3d.py)

### External tool interfaces

The CLI passes arguments separately through `std::process::Command`.

- `P3DDebin.exe debin input.p3d -o <folder>` produces `input_debin.p3d`. Version 1.2.1 was tested with ODOL 75. [Tested release](https://github.com/Wesley-TB/P3DDebinarizer/releases/tag/v1.2.1)
- `P3DDebinarizer.exe input.p3d` is expected to produce `input_mlod.p3d`. The archived ScripyZz release reported “Unknown ODOL version” for the same model in an interactive console. It also failed with redirected console output. [Archived repository](https://github.com/ScripyZz/P3D-Debinarizer-Arma-3)
- `hemtt utils paa convert input.paa output.png` converts the first mipmap. HEMTT 1.23.1 was tested. [HEMTT command](https://hemtt.dev/utilities/paa/convert.html)
- Any other executable name selects the generic `tool input output` interface with two absolute paths. A local adapter can use that interface to call software with different arguments.

## Arma 3 reports and exit codes

The top-level `report.json` records Workshop metadata, the cached path, the detected format, each PBO's prefix and file list, model directories and statuses, and batch errors. Each model directory has its own `report.json`. Successful exports also include `model.obj`, `model.mtl` and the available PNG textures. The report maps OBJ material names back to Arma asset references.

The JSON status values are `ok` (success), `partiel` (partial export) and `échec` (failure). A corrupt file or unknown version allows subsequent models to continue converting. Exit code 0 means the whole batch succeeded. Exit code 2 means the batch was incomplete or refused. A setup or write error that prevents report creation goes directly to stderr.

## Arma 3 validation

| Stage | Observed result | Coverage limit |
|---|---|---|
| Build and tests | Stable Rust 1.100, edition 2021, Windows x64 GNU. The release build and all six integration tests passed. | Linux and macOS were not tested. |
| Steam | Cargo tests covered numeric IDs and URLs. A live CLI request recorded metadata for the supplied Workshop item and stopped on an empty file URL. | A successful Steam payload download and cached revision reuse still need a live mod test. |
| Extraction | The ACE3 3.21.3 release PBO `ace_dogtags.pbo` yielded 48 files and a resolved virtual prefix. Its SHA-256 was identical before and after conversion. Synthetic fixtures covered ZIP-wrapped PBO, Cprs, invalid checksum, truncation and invalid paths. | Cprs compression was checked with synthetic data. |
| Geometry | The reader processed the ACE3 source MLOD `ace_dogtag.p3d` and independently debinarized the distributed ODOL v75. Both produced LOD 1 with 28 vertices and 26 faces. | Other ODOL versions and the geometric fidelity of complex models remain unverified. |
| Materials and Blender | A real PAA produced a 256 × 256 PNG. Blender 4.4.3 imported the OBJ with 28 vertices and 26 faces, loaded the PNG and rendered the tag with its transparent hole visible. | This visual check covers a flat tag. In-game shaders and orientation on complex asymmetric geometry remain unverified. |
| Error recovery | One synthetic batch contained an unknown ODOL version, a truncated MLOD and a valid model with a missing texture. The first two failed individually. The third produced OBJ/MTL and a partial report. Other checks covered PNG fallback, LOD selection, material grouping and UV indices. | Proxy and animation tags and HitPoints LOD exclusion were checked with synthetic data. |

LOD selection and proxy identification rely on P3DM conventions. Named selections can represent entities other than bones. Classes found in a model.cfg describe the declarations available in its directory, subject to the inheritance limits above. The local validation render and reports are under `sortie/validation-ace3/`. [Tested ACE3 release](https://github.com/acemod/ACE3/releases/tag/v3.21.3) · [Source MLOD](https://github.com/acemod/ACE3/blob/a6c876718e5e86016a41ffd050d584a305633e5b/addons/dogtags/data/ace_dogtag.p3d)

## Direct Rust dependencies

Clap handles arguments and environment variables. Blocking reqwest uses native TLS. Serde and serde_json read Steam responses and write reports. Zip reads ZIP containers, image validates and re-encodes PNGs, and anyhow propagates error context. Tempfile provides isolated working directories and file creation without overwriting existing files. Binary parsing uses bounds-checked slices and the standard library's `from_le_bytes`. The project forbids `unsafe` in its Rust code.

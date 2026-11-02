# Linux release identity audit

The supplied Linux installation metadata matches Valve's current public branch as observed on 2026-11-02. Calling it an outdated Linux installation was unsupported. The Linux/Windows `CombatModifiers` difference establishes an incompatible platform layout; it does not establish when the Linux installation was released or whether a newer Linux release exists.

[Privacy-filtered comparison](../evidence/linux/version-current-public.json) records an anonymous, live Valve SteamCMD `app_info_update 1` / `app_info_print 289070` response. The installed and public build IDs are both `15296837`. **All 20 installed depot manifest IDs match the corresponding current public depot manifests**, including every installed Linux-specific depot. Principal examples:

| Depot | Installed and current public manifest |
| --- | --- |
| 533502 | 731801825977417543 |
| 533503 | 4174395120133247913 |
| 947519 | 5584778302079664846 |

The public branch timestamp is `2024-08-07T23:07:34Z`. That dates the branch metadata, not necessarily each platform's binary. The supplied Aspyr version file reports `1.0.12.9`, changelist `363760`. Installation files have separately recorded content hashes; matching installation metadata does not verify every copied file against a downloaded Valve depot manifest.

The XP2 ELF's exported `GameCore::XP2::Context::CIV6_DLL_VERSION` points to the string `1.0.0`. Static inspection finds both context `GetDLLVersion` implementations read that constant (export wrapper VA `0x2c7c820`, instance implementation VA `0x2d76ae0`, constant VA `0x3c00668`, string VA `0x2f8dfbc`). This generic module version is not a useful release date or `1.0.12.x` identity. These addresses apply only to SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; no original code was executed for this audit.

The [Aspyr Linux FAQ](https://support.aspyr.com/hc/en-us/articles/360020392091-Civilization-VI-Linux-FAQ) says Windows/Linux multiplayer is available when versions are in sync. Its October 2022 update and lack of an exact current build number cannot establish today's depot status. The [Leader Pass FAQ](https://support.aspyr.com/hc/en-us/articles/10509184897933-Civilization-VI-Leader-Pass-FAQ) likewise does not identify the current Linux binary. Valve's live depot identities provide the narrower installation comparison above. No third-party release chronology is needed for that conclusion.

Reproduce using [Valve SteamCMD](https://developer.valvesoftware.com/wiki/SteamCMD), anonymously and without downloading game content:

```sh
steamcmd +login anonymous +app_info_update 1 +app_info_print 289070 +quit > /tmp/c6-appinfo.log
python3 tools/linux/version_audit.py ../c6-assets-linux /tmp/c6-appinfo.log \
  --observed-at ACTUAL-UTC-TIMESTAMP --output /tmp/c6-version-comparison.json
```

The tool excludes Steam account metadata. It requires a freshly retrieved response to support a currentness claim. This audit does not establish cross-platform gameplay equivalence, an initialized game, legal actions, or full turns; the separate hash-gated Windows acceptance target remains unchanged.

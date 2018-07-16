// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use client::Api;
use failure::Error;
use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::{install::Install, State};
use std::fs;
use update_package::{ObjectStatus, UpdatePackage};
use walkdir::WalkDir;

#[derive(Debug, PartialEq)]
pub(crate) struct Download {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
    pub(crate) update_package: UpdatePackage,
}

impl State for Download {
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.settings;
        let runtime_settings = s.runtime_settings;
        let firmware = s.firmware;
        let update_package = s.update_package;

        // Prune left over from previous installations
        for entry in WalkDir::new(&settings.update.download_dir)
            .follow_links(true)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_file())
            .filter_map(|e| e.ok())
            .filter(|e| {
                !update_package
                    .objects()
                    .iter()
                    .map(|o| o.sha256sum())
                    .collect::<Vec<_>>()
                    .contains(&e.file_name().to_str().unwrap_or(""))
            }) {
            fs::remove_file(entry.path())?;
        }

        // Prune corrupted files
        for object in update_package.filter_objects(&settings, &ObjectStatus::Corrupted) {
            fs::remove_file(&settings.update.download_dir.join(object.sha256sum()))?;
        }

        // Download the missing or incomplete objects
        for object in update_package
            .filter_objects(&settings, &ObjectStatus::Missing)
            .into_iter()
            .chain(update_package.filter_objects(&settings, &ObjectStatus::Incomplete))
        {
            Api::new(&settings, &runtime_settings, &firmware)
                .download_object(&update_package.package_uid(), object.sha256sum())?;
        }

        if update_package
            .objects()
            .iter()
            .all(|o| o.status(&settings.update.download_dir).ok() == Some(ObjectStatus::Ready))
        {
            Ok(Box::new(Install {
                settings,
                runtime_settings,
                firmware,
                update_package,
            }))
        } else {
            bail!("Not all objects are ready for use")
        }
    }
}

#[test]
fn skip_download_if_ready() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs::create_dir_all;
    use update_package::tests::{create_fake_object, create_fake_settings, get_update_package};

    let settings = create_fake_settings();
    let tmpdir = settings.update.download_dir.clone();
    let _ = create_dir_all(&tmpdir);
    let _ = create_fake_object(&settings);

    let machine = Box::new(Download {
        settings: settings,
        runtime_settings: RuntimeSettings::default(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        update_package: get_update_package(),
    }).handle();

    assert_state!(machine, Install);

    assert_eq!(
        WalkDir::new(&tmpdir)
            .follow_links(true)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_file())
            .count(),
        1,
        "Number of objects is wrong"
    );
}

#[test]
fn download_objects() {
    use super::*;
    use crypto_hash::{hex_digest, Algorithm};
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use mockito::mock;
    use std::fs::create_dir_all;
    use std::fs::File;
    use std::io::Read;
    use update_package::tests::{create_fake_settings, get_update_package};

    let settings = create_fake_settings();
    let update_package = get_update_package();
    let sha256sum = "c775e7b757ede630cd0aa1113bd102661ab38829ca52a6422ab782862f268646";
    let tmpdir = settings.update.download_dir.clone();
    let _ = create_dir_all(&tmpdir);

    // leftover file to ensure it is removed
    let _ = File::create(&tmpdir.join("leftover-file"));

    let mock = mock(
        "GET",
        format!(
            "/products/{}/packages/{}/objects/{}",
            "229ffd7e08721d716163fc81a2dbaf6c90d449f0a3b009b6a2defe8a0b0d7381",
            &update_package.package_uid(),
            &sha256sum
        ).as_str(),
    ).match_header("Content-Type", "application/json")
        .match_header("Api-Content-Type", "application/vnd.updatehub-v1+json")
        .with_status(200)
        .with_body("1234567890")
        .create();

    let machine = Box::new(Download {
        settings: settings,
        runtime_settings: RuntimeSettings::default(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        update_package: update_package,
    }).handle();

    mock.assert();

    assert_state!(machine, Install);

    assert_eq!(
        WalkDir::new(&tmpdir)
            .follow_links(true)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_file())
            .count(),
        1,
        "Failed to remove the corrupted object"
    );

    let mut object_content = String::new();
    let _ = File::open(&tmpdir.join(&sha256sum))
        .expect("Fail to open the temporary directory.")
        .read_to_string(&mut object_content);

    assert_eq!(
        &hex_digest(Algorithm::SHA256, object_content.as_bytes()),
        &sha256sum,
        "Checksum mismatch"
    );
}

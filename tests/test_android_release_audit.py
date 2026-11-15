import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
import zipfile
import sys
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('android_release_audit', Path(__file__).resolve().parents[1] / 'tools/check_android_release.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def archive(files):
    output = io.BytesIO()
    with zipfile.ZipFile(output, 'w') as z:
        for name, data in files.items():
            z.writestr(name, data)
    return output.getvalue()


class ReleaseAuditTests(unittest.TestCase):
    def test_dev_package_does_not_relax_stable_identity_or_debuggable_check(self):
        with tempfile.TemporaryDirectory() as temporary:
            apk = Path(temporary) / 'test.apk'
            apk.write_bytes(b'test')
            payload = {'findings': []}
            with patch.object(audit, 'inspect_payload', return_value=payload), patch.object(
                    audit.subprocess, 'check_output', return_value="package: name='dev.horizon.android.dev'\n"):
                with self.assertRaises(ValueError):
                    audit.audit(apk, 'aapt2')
                report = audit.audit(apk, 'aapt2', app_id='dev.horizon.android.dev')
                self.assertTrue(report['passed'])
            with patch.object(audit, 'inspect_payload', return_value={'findings': []}), patch.object(
                    audit.subprocess, 'check_output', return_value="package: name='dev.horizon.android.dev'\napplication-debuggable\n"):
                self.assertFalse(audit.audit(apk, 'aapt2', app_id='dev.horizon.android.dev')['passed'])

    def test_public_apk_accepts_generated_tables_and_rejects_original_assets(self):
        build_spec = importlib.util.spec_from_file_location('android_build', Path(__file__).resolve().parents[1] / 'tools/android_build.py')
        build = importlib.util.module_from_spec(build_spec)
        build_spec.loader.exec_module(build)
        self.assertTrue(build.generated_engine_file('worldgen/blocks/structure-blocks.json'))
        self.assertFalse(build.generated_engine_file('worldgen/structure/minecraft/village/test.nbt'))
        # 1.8.9's generated item definitions and frames ship; the jar's own
        # models and textures are downloaded instead.
        self.assertTrue(build.generated_engine_file('assets/minecraft/items/diamond_sword.json'))
        self.assertTrue(build.generated_engine_file('assets/minecraft/models/item/spawn_egg_base.json'))
        self.assertTrue(build.generated_engine_file('assets/minecraft/models/item/clock_63.json'))
        self.assertTrue(build.generated_engine_file('assets/minecraft/textures/items/compass_31.png'))
        self.assertFalse(build.generated_engine_file('assets/minecraft/models/item/diamond_sword.json'))
        self.assertFalse(build.generated_engine_file('assets/minecraft/textures/items/compass.png'))
        # The 1.8 chest tables are generated from the 1.8 jar's own lists;
        # every other loot table is still the 26.3 data pack's.
        self.assertTrue(build.generated_engine_file('data/minecraft/loot_table/chests/simple_dungeon.json'))
        self.assertTrue(build.generated_engine_file('data/minecraft/loot_table/chests/village/village_weaponsmith.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/loot_table/chests/end_city_treasure.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/loot_table/entities/zombie.json'))
        # Recipes are 1.8's RecipeDispatcher dump; a 26.3 recipe is not.
        self.assertTrue(build.generated_engine_file('data/minecraft/recipe/1_8_9_0000_wooden_pickaxe.json'))
        self.assertTrue(build.generated_engine_file('data/minecraft/recipe/1_8_9_banner_pattern.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/recipe/wooden_pickaxe.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/recipe/armor_dye.json'))
        # Paintings are 1.8's enum; 26.3's extra motives have no 1.8 texture.
        self.assertTrue(build.generated_engine_file('data/minecraft/painting_variant/kebab.json'))
        self.assertTrue(build.generated_engine_file('data/minecraft/painting_variant/aztec2.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/painting_variant/meditative.json'))
        self.assertTrue(build.generated_engine_file('data/minecraft/tags/painting_variant/placeable.json'))
        # The per-motive textures and back are cropped from the 1.8 atlas; the
        # downloaded jar keeps the atlas and 26.3's extra art stays out.
        self.assertTrue(build.generated_engine_file('assets/minecraft/textures/painting/kebab.png'))
        self.assertTrue(build.generated_engine_file('assets/minecraft/textures/painting/donkey_kong.png'))
        self.assertTrue(build.generated_engine_file('assets/minecraft/textures/painting/back.png'))
        self.assertFalse(build.generated_engine_file('assets/minecraft/textures/painting/meditative.png'))
        self.assertFalse(build.generated_engine_file('assets/minecraft/textures/painting/paintings_kristoffer_zetterstrand.png'))
        # Banners are 1.8's 39 enum patterns and their six item tags; the four
        # post-1.8 patterns have no 1.8 code or art.
        self.assertTrue(build.generated_engine_file('data/minecraft/banner_pattern/creeper.json'))
        self.assertTrue(build.generated_engine_file('data/minecraft/banner_pattern/diagonal_up_right.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/banner_pattern/flow.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/banner_pattern/globe.json'))
        self.assertTrue(build.generated_engine_file('data/minecraft/tags/banner_pattern/no_item_required.json'))
        self.assertTrue(build.generated_engine_file('data/minecraft/tags/banner_pattern/pattern_item/bordure_indented.json'))
        self.assertFalse(build.generated_engine_file('data/minecraft/tags/banner_pattern/pattern_item/flow.json'))
        # The converted masks and the renamed pole-and-cloth texture are
        # generated; the jar's white-on-black masks and old name are not.
        self.assertTrue(build.generated_engine_file('assets/minecraft/textures/entity/banner/creeper.png'))
        self.assertTrue(build.generated_engine_file('assets/minecraft/textures/entity/banner/banner_base.png'))
        self.assertFalse(build.generated_engine_file('assets/minecraft/textures/entity/banner/flow.png'))
        self.assertFalse(build.generated_engine_file('assets/minecraft/textures/entity/banner/base.png'))
        self.assertFalse(build.generated_engine_file('assets/minecraft/textures/entity/banner_base.png'))
        with tempfile.TemporaryDirectory() as folder, patch.dict(sys.modules, {'android_build': build}):
            path = Path(folder) / 'public.apk'
            files = {'AndroidManifest.xml': b'test', 'assets/engine-data.zip': archive({'facts.json': b'{}'})}
            path.write_bytes(archive(files))
            self.assertEqual(audit.inspect_payload(path, public=True)['findings'], [])
            for name in ['worldgen/structure/minecraft/test.nbt', 'worldgen/biomes.json', 'raw.jar', 'unknown.json']:
                files['assets/engine-data.zip'] = archive({name: b'original content'})
                path.write_bytes(archive(files))
                self.assertIn('original_game_resource', {f['kind'] for f in audit.inspect_payload(path, public=True)['findings']})
            files['assets/bedrock-controls.zip'] = archive({'ui/jump.png': b'texture'})
            path.write_bytes(archive(files))
            with self.assertRaises(ValueError):
                audit.inspect_payload(path, public=True)
    def inspect(self, engine=None, extra=None):
        files = {'AndroidManifest.xml': b'test manifest',
                 'assets/engine-data.zip': archive(engine or {'blocks.json': b'{}'}),
                 'assets/bedrock-controls.zip': archive({'LICENSE.md': b'license'}),
                 **(extra or {})}
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'test.apk'
            path.write_bytes(archive(files))
            return audit.inspect_payload(path)

    def test_resource_only_apk_has_no_private_findings(self):
        self.assertEqual(self.inspect()['findings'], [])

    def test_androidx_build_profiles_are_allowed(self):
        self.assertEqual(self.inspect(extra={'assets/dexopt/baseline.prof': b'compiled profile'})['findings'], [])

    def test_nested_player_data_and_unexpected_asset_are_rejected(self):
        report = self.inspect({'profiles/account.json': b'{}'}, {'assets/settings.json': b'{}'})
        self.assertEqual({f['kind'] for f in report['findings']}, {'private_file', 'unexpected_asset'})

    def test_credential_in_native_library_is_rejected_without_reporting_value(self):
        token = b'ghp_' + b'X' * 36
        report = self.inspect(extra={'lib/arm64-v8a/libtest.so': b'ELF\x00' + token})
        self.assertEqual(report['findings'][0]['kind'], 'github_token')
        self.assertNotIn(token.decode(), str(report))

    def test_personal_build_path_and_private_key_are_rejected(self):
        key = b'-----BEGIN ' + b'PRIVATE KEY-----'
        report = self.inspect(extra={'lib/arm64-v8a/libtest.so': b'C:/Users/example/.cargo/source.rs\x00' + key})
        self.assertEqual({f['kind'] for f in report['findings']}, {'personal_build_path', 'private_key'})

    def test_nested_traversal_is_rejected(self):
        with self.assertRaises(ValueError):
            self.inspect({'../account.json': b'{}'})

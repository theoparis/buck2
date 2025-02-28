/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

package com.facebook.buck.android.resources;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;

import com.facebook.buck.testutil.MoreAsserts;
import com.facebook.buck.testutil.TemporaryPaths;
import com.facebook.buck.testutil.integration.TestDataHelper;
import com.google.common.collect.ImmutableList;
import com.google.common.io.ByteStreams;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;
import java.util.zip.ZipFile;
import org.junit.Before;
import org.junit.Rule;
import org.junit.Test;

public class ResTableTypeSpecTest {
  private static final String APK_NAME = "example.apk";

  @Rule public TemporaryPaths tmpFolder = new TemporaryPaths();
  private Path apkPath;

  @Before
  public void setUp() {
    apkPath = TestDataHelper.getTestDataDirectory(this).resolve("aapt_dump").resolve(APK_NAME);
  }

  @Test
  public void testGetAndSerialize() throws Exception {
    try (ZipFile apkZip = new ZipFile(apkPath.toFile())) {
      ByteBuffer buf =
          ResChunk.wrap(
              ByteStreams.toByteArray(apkZip.getInputStream(apkZip.getEntry("resources.arsc"))));

      List<Integer> offsets = ChunkUtils.findChunks(buf, ResChunk.CHUNK_RES_TABLE_TYPE_SPEC);
      assertEquals(ImmutableList.of(1024, 1040, 1320, 1436, 1624, 1896), offsets);

      int expectedOffset = 1024;
      for (int offset : offsets) {
        assertEquals(expectedOffset, offset);
        ByteBuffer data = ResChunk.slice(buf, offset);
        ResTableTypeSpec resSpec = ResTableTypeSpec.get(data);

        byte[] expected =
            Arrays.copyOfRange(
                data.array(), data.arrayOffset(), data.arrayOffset() + resSpec.getTotalSize());
        byte[] actual = resSpec.serialize();

        assertArrayEquals(expected, actual);
        expectedOffset += resSpec.getTotalSize();
      }
    }
  }

  @Test
  public void testFullSliceResTableTypeSpec() throws Exception {
    try (ZipFile apkZip = new ZipFile(apkPath.toFile())) {
      ByteBuffer buf =
          ResChunk.wrap(
              ByteStreams.toByteArray(apkZip.getInputStream(apkZip.getEntry("resources.arsc"))));

      ResourceTable resourceTable = ResourceTable.get(buf);
      ResTablePackage resPackage = resourceTable.getPackage();
      for (ResTableTypeSpec spec : resPackage.getTypeSpecs()) {
        int entryCount = spec.getEntryCount();
        ResTableTypeSpec copy = ResTableTypeSpec.slice(spec, entryCount);

        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        spec.dump(resourceTable.getStrings(), resPackage, new PrintStream(baos));
        String expected = new String(baos.toByteArray(), StandardCharsets.UTF_8);

        baos = new ByteArrayOutputStream();
        copy.dump(resourceTable.getStrings(), resPackage, new PrintStream(baos));
        String content = new String(baos.toByteArray(), StandardCharsets.UTF_8);

        MoreAsserts.assertLargeStringsEqual(expected, content);
      }
    }
  }
}

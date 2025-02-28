/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

package com.facebook.buck.jvm.java;

import com.facebook.buck.core.util.immutables.BuckStyleValue;
import com.facebook.buck.jvm.java.abi.source.api.SourceOnlyAbiRuleInfoFactory;
import javax.tools.JavaFileManager;

/** Default factory for SourceOnlyAbiRuleInfos. */
@BuckStyleValue
public abstract class DefaultSourceOnlyAbiRuleInfoFactory implements SourceOnlyAbiRuleInfoFactory {

  abstract String getFullyQualifiedBuildTargetName();

  abstract boolean isRuleIsRequiredForSourceOnlyAbi();

  public static DefaultSourceOnlyAbiRuleInfoFactory of(
      String fullyQualifiedBuildTargetName, boolean ruleIsRequiredForSourceOnlyAbi) {
    return ImmutableDefaultSourceOnlyAbiRuleInfoFactory.ofImpl(
        fullyQualifiedBuildTargetName, ruleIsRequiredForSourceOnlyAbi);
  }

  @Override
  public SourceOnlyAbiRuleInfo create(JavaFileManager fileManager) {
    return new DefaultSourceOnlyAbiRuleInfo(
        fileManager, getFullyQualifiedBuildTargetName(), isRuleIsRequiredForSourceOnlyAbi());
  }
}

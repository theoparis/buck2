// @generated
// Generated by the protocol buffer compiler.  DO NOT EDIT!
// source: kotlincd.proto

// Protobuf Java Version: 3.25.6
package com.facebook.buck.cd.model.kotlin;

@javax.annotation.Generated(value="protoc", comments="annotations:MetadataOrBuilder.java.pb.meta")
public interface MetadataOrBuilder extends
    // @@protoc_insertion_point(interface_extends:kotlincd.api.v1.Metadata)
    com.google.protobuf.MessageOrBuilder {

  /**
   * <code>repeated .kotlincd.api.v1.Digests digests = 1;</code>
   */
  java.util.List<com.facebook.buck.cd.model.kotlin.Digests> 
      getDigestsList();
  /**
   * <code>repeated .kotlincd.api.v1.Digests digests = 1;</code>
   */
  com.facebook.buck.cd.model.kotlin.Digests getDigests(int index);
  /**
   * <code>repeated .kotlincd.api.v1.Digests digests = 1;</code>
   */
  int getDigestsCount();
  /**
   * <code>repeated .kotlincd.api.v1.Digests digests = 1;</code>
   */
  java.util.List<? extends com.facebook.buck.cd.model.kotlin.DigestsOrBuilder> 
      getDigestsOrBuilderList();
  /**
   * <code>repeated .kotlincd.api.v1.Digests digests = 1;</code>
   */
  com.facebook.buck.cd.model.kotlin.DigestsOrBuilder getDigestsOrBuilder(
      int index);
}

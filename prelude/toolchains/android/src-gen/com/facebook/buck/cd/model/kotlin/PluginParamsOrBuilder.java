// @generated
// Generated by the protocol buffer compiler.  DO NOT EDIT!
// source: kotlincd.proto

package com.facebook.buck.cd.model.kotlin;

@javax.annotation.Generated(value="protoc", comments="annotations:PluginParamsOrBuilder.java.pb.meta")
public interface PluginParamsOrBuilder extends
    // @@protoc_insertion_point(interface_extends:kotlincd.api.v1.PluginParams)
    com.google.protobuf.MessageOrBuilder {

  /**
   * <code>map&lt;string, string&gt; params = 1;</code>
   */
  int getParamsCount();
  /**
   * <code>map&lt;string, string&gt; params = 1;</code>
   */
  boolean containsParams(
      java.lang.String key);
  /**
   * Use {@link #getParamsMap()} instead.
   */
  @java.lang.Deprecated
  java.util.Map<java.lang.String, java.lang.String>
  getParams();
  /**
   * <code>map&lt;string, string&gt; params = 1;</code>
   */
  java.util.Map<java.lang.String, java.lang.String>
  getParamsMap();
  /**
   * <code>map&lt;string, string&gt; params = 1;</code>
   */

  java.lang.String getParamsOrDefault(
      java.lang.String key,
      java.lang.String defaultValue);
  /**
   * <code>map&lt;string, string&gt; params = 1;</code>
   */

  java.lang.String getParamsOrThrow(
      java.lang.String key);
}

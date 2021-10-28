//Generated by the protocol buffer compiler. DO NOT EDIT!
// source: services/feed/feed.proto

package qaul.rpc.feed;

@kotlin.jvm.JvmSynthetic
inline fun feedMessageRequest(block: qaul.rpc.feed.FeedMessageRequestKt.Dsl.() -> Unit): qaul.rpc.feed.FeedOuterClass.FeedMessageRequest =
  qaul.rpc.feed.FeedMessageRequestKt.Dsl._create(qaul.rpc.feed.FeedOuterClass.FeedMessageRequest.newBuilder()).apply { block() }._build()
object FeedMessageRequestKt {
  @kotlin.OptIn(com.google.protobuf.kotlin.OnlyForUseByGeneratedProtoCode::class)
  @com.google.protobuf.kotlin.ProtoDslMarker
  class Dsl private constructor(
    @kotlin.jvm.JvmField private val _builder: qaul.rpc.feed.FeedOuterClass.FeedMessageRequest.Builder
  ) {
    companion object {
      @kotlin.jvm.JvmSynthetic
      @kotlin.PublishedApi
      internal fun _create(builder: qaul.rpc.feed.FeedOuterClass.FeedMessageRequest.Builder): Dsl = Dsl(builder)
    }

    @kotlin.jvm.JvmSynthetic
    @kotlin.PublishedApi
    internal fun _build(): qaul.rpc.feed.FeedOuterClass.FeedMessageRequest = _builder.build()

    /**
     * <pre>
     * message id of the last received message
     * this can be empty, then all last messages
     * are sent.
     * </pre>
     *
     * <code>bytes last_received = 1;</code>
     */
    var lastReceived: com.google.protobuf.ByteString
      @JvmName("getLastReceived")
      get() = _builder.getLastReceived()
      @JvmName("setLastReceived")
      set(value) {
        _builder.setLastReceived(value)
      }
    /**
     * <pre>
     * message id of the last received message
     * this can be empty, then all last messages
     * are sent.
     * </pre>
     *
     * <code>bytes last_received = 1;</code>
     */
    fun clearLastReceived() {
      _builder.clearLastReceived()
    }
  }
}
@kotlin.jvm.JvmSynthetic
inline fun qaul.rpc.feed.FeedOuterClass.FeedMessageRequest.copy(block: qaul.rpc.feed.FeedMessageRequestKt.Dsl.() -> Unit): qaul.rpc.feed.FeedOuterClass.FeedMessageRequest =
  qaul.rpc.feed.FeedMessageRequestKt.Dsl._create(this.toBuilder()).apply { block() }._build()
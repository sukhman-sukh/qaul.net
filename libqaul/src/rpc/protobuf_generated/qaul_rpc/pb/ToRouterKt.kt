//Generated by the protocol buffer compiler. DO NOT EDIT!
// source: to_libqaul.proto

package qaul_rpc.pb;

@kotlin.jvm.JvmSynthetic
inline fun toRouter(block: qaul_rpc.pb.ToRouterKt.Dsl.() -> Unit): qaul_rpc.pb.ToLibqaulOuterClass.ToRouter =
  qaul_rpc.pb.ToRouterKt.Dsl._create(qaul_rpc.pb.ToLibqaulOuterClass.ToRouter.newBuilder()).apply { block() }._build()
object ToRouterKt {
  @kotlin.OptIn(com.google.protobuf.kotlin.OnlyForUseByGeneratedProtoCode::class)
  @com.google.protobuf.kotlin.ProtoDslMarker
  class Dsl private constructor(
    @kotlin.jvm.JvmField private val _builder: qaul_rpc.pb.ToLibqaulOuterClass.ToRouter.Builder
  ) {
    companion object {
      @kotlin.jvm.JvmSynthetic
      @kotlin.PublishedApi
      internal fun _create(builder: qaul_rpc.pb.ToLibqaulOuterClass.ToRouter.Builder): Dsl = Dsl(builder)
    }

    @kotlin.jvm.JvmSynthetic
    @kotlin.PublishedApi
    internal fun _build(): qaul_rpc.pb.ToLibqaulOuterClass.ToRouter = _builder.build()

    /**
     * <code>.qaul_rpc.pb.RequestUsers request_users = 1;</code>
     */
    var requestUsers: qaul_rpc.pb.ToLibqaulOuterClass.RequestUsers
      @JvmName("getRequestUsers")
      get() = _builder.getRequestUsers()
      @JvmName("setRequestUsers")
      set(value) {
        _builder.setRequestUsers(value)
      }
    /**
     * <code>.qaul_rpc.pb.RequestUsers request_users = 1;</code>
     */
    fun clearRequestUsers() {
      _builder.clearRequestUsers()
    }
    /**
     * <code>.qaul_rpc.pb.RequestUsers request_users = 1;</code>
     * @return Whether the requestUsers field is set.
     */
    fun hasRequestUsers(): kotlin.Boolean {
      return _builder.hasRequestUsers()
    }
    val typeCase: qaul_rpc.pb.ToLibqaulOuterClass.ToRouter.TypeCase
      @JvmName("getTypeCase")
      get() = _builder.getTypeCase()

    fun clearType() {
      _builder.clearType()
    }
  }
}
@kotlin.jvm.JvmSynthetic
inline fun qaul_rpc.pb.ToLibqaulOuterClass.ToRouter.copy(block: qaul_rpc.pb.ToRouterKt.Dsl.() -> Unit): qaul_rpc.pb.ToLibqaulOuterClass.ToRouter =
  qaul_rpc.pb.ToRouterKt.Dsl._create(this.toBuilder()).apply { block() }._build()
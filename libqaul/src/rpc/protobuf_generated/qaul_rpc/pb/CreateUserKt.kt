//Generated by the protocol buffer compiler. DO NOT EDIT!
// source: to_libqaul.proto

package qaul_rpc.pb;

@kotlin.jvm.JvmSynthetic
inline fun createUser(block: qaul_rpc.pb.CreateUserKt.Dsl.() -> Unit): qaul_rpc.pb.ToLibqaulOuterClass.CreateUser =
  qaul_rpc.pb.CreateUserKt.Dsl._create(qaul_rpc.pb.ToLibqaulOuterClass.CreateUser.newBuilder()).apply { block() }._build()
object CreateUserKt {
  @kotlin.OptIn(com.google.protobuf.kotlin.OnlyForUseByGeneratedProtoCode::class)
  @com.google.protobuf.kotlin.ProtoDslMarker
  class Dsl private constructor(
    @kotlin.jvm.JvmField private val _builder: qaul_rpc.pb.ToLibqaulOuterClass.CreateUser.Builder
  ) {
    companion object {
      @kotlin.jvm.JvmSynthetic
      @kotlin.PublishedApi
      internal fun _create(builder: qaul_rpc.pb.ToLibqaulOuterClass.CreateUser.Builder): Dsl = Dsl(builder)
    }

    @kotlin.jvm.JvmSynthetic
    @kotlin.PublishedApi
    internal fun _build(): qaul_rpc.pb.ToLibqaulOuterClass.CreateUser = _builder.build()

    /**
     * <code>string name = 1;</code>
     */
    var name: kotlin.String
      @JvmName("getName")
      get() = _builder.getName()
      @JvmName("setName")
      set(value) {
        _builder.setName(value)
      }
    /**
     * <code>string name = 1;</code>
     */
    fun clearName() {
      _builder.clearName()
    }
  }
}
@kotlin.jvm.JvmSynthetic
inline fun qaul_rpc.pb.ToLibqaulOuterClass.CreateUser.copy(block: qaul_rpc.pb.CreateUserKt.Dsl.() -> Unit): qaul_rpc.pb.ToLibqaulOuterClass.CreateUser =
  qaul_rpc.pb.CreateUserKt.Dsl._create(this.toBuilder()).apply { block() }._build()
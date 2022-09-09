//Generated by the protocol buffer compiler. DO NOT EDIT!
// source: services/group/group_net.proto

package qaul.net.group;

@kotlin.jvm.JvmName("-initializegroupInfo")
inline fun groupInfo(block: qaul.net.group.GroupInfoKt.Dsl.() -> kotlin.Unit): qaul.net.group.GroupNet.GroupInfo =
  qaul.net.group.GroupInfoKt.Dsl._create(qaul.net.group.GroupNet.GroupInfo.newBuilder()).apply { block() }._build()
object GroupInfoKt {
  @kotlin.OptIn(com.google.protobuf.kotlin.OnlyForUseByGeneratedProtoCode::class)
  @com.google.protobuf.kotlin.ProtoDslMarker
  class Dsl private constructor(
    private val _builder: qaul.net.group.GroupNet.GroupInfo.Builder
  ) {
    companion object {
      @kotlin.jvm.JvmSynthetic
      @kotlin.PublishedApi
      internal fun _create(builder: qaul.net.group.GroupNet.GroupInfo.Builder): Dsl = Dsl(builder)
    }

    @kotlin.jvm.JvmSynthetic
    @kotlin.PublishedApi
    internal fun _build(): qaul.net.group.GroupNet.GroupInfo = _builder.build()

    /**
     * <pre>
     * group id
     * </pre>
     *
     * <code>bytes group_id = 1;</code>
     */
    var groupId: com.google.protobuf.ByteString
      @JvmName("getGroupId")
      get() = _builder.getGroupId()
      @JvmName("setGroupId")
      set(value) {
        _builder.setGroupId(value)
      }
    /**
     * <pre>
     * group id
     * </pre>
     *
     * <code>bytes group_id = 1;</code>
     */
    fun clearGroupId() {
      _builder.clearGroupId()
    }

    /**
     * <pre>
     * group name
     * </pre>
     *
     * <code>string group_name = 2;</code>
     */
    var groupName: kotlin.String
      @JvmName("getGroupName")
      get() = _builder.getGroupName()
      @JvmName("setGroupName")
      set(value) {
        _builder.setGroupName(value)
      }
    /**
     * <pre>
     * group name
     * </pre>
     *
     * <code>string group_name = 2;</code>
     */
    fun clearGroupName() {
      _builder.clearGroupName()
    }

    /**
     * <pre>
     * created at
     * </pre>
     *
     * <code>uint64 created_at = 3;</code>
     */
    var createdAt: kotlin.Long
      @JvmName("getCreatedAt")
      get() = _builder.getCreatedAt()
      @JvmName("setCreatedAt")
      set(value) {
        _builder.setCreatedAt(value)
      }
    /**
     * <pre>
     * created at
     * </pre>
     *
     * <code>uint64 created_at = 3;</code>
     */
    fun clearCreatedAt() {
      _builder.clearCreatedAt()
    }

    /**
     * <pre>
     * group revision
     * </pre>
     *
     * <code>uint32 revision = 4;</code>
     */
    var revision: kotlin.Int
      @JvmName("getRevision")
      get() = _builder.getRevision()
      @JvmName("setRevision")
      set(value) {
        _builder.setRevision(value)
      }
    /**
     * <pre>
     * group revision
     * </pre>
     *
     * <code>uint32 revision = 4;</code>
     */
    fun clearRevision() {
      _builder.clearRevision()
    }

    /**
     * An uninstantiable, behaviorless type to represent the field in
     * generics.
     */
    @kotlin.OptIn(com.google.protobuf.kotlin.OnlyForUseByGeneratedProtoCode::class)
    class MembersProxy private constructor() : com.google.protobuf.kotlin.DslProxy()
    /**
     * <pre>
     * updated members
     * </pre>
     *
     * <code>repeated .qaul.net.group.GroupMember members = 5;</code>
     */
     val members: com.google.protobuf.kotlin.DslList<qaul.net.group.GroupNet.GroupMember, MembersProxy>
      @kotlin.jvm.JvmSynthetic
      get() = com.google.protobuf.kotlin.DslList(
        _builder.getMembersList()
      )
    /**
     * <pre>
     * updated members
     * </pre>
     *
     * <code>repeated .qaul.net.group.GroupMember members = 5;</code>
     * @param value The members to add.
     */
    @kotlin.jvm.JvmSynthetic
    @kotlin.jvm.JvmName("addMembers")
    fun com.google.protobuf.kotlin.DslList<qaul.net.group.GroupNet.GroupMember, MembersProxy>.add(value: qaul.net.group.GroupNet.GroupMember) {
      _builder.addMembers(value)
    }
    /**
     * <pre>
     * updated members
     * </pre>
     *
     * <code>repeated .qaul.net.group.GroupMember members = 5;</code>
     * @param value The members to add.
     */
    @kotlin.jvm.JvmSynthetic
    @kotlin.jvm.JvmName("plusAssignMembers")
    @Suppress("NOTHING_TO_INLINE")
    inline operator fun com.google.protobuf.kotlin.DslList<qaul.net.group.GroupNet.GroupMember, MembersProxy>.plusAssign(value: qaul.net.group.GroupNet.GroupMember) {
      add(value)
    }
    /**
     * <pre>
     * updated members
     * </pre>
     *
     * <code>repeated .qaul.net.group.GroupMember members = 5;</code>
     * @param values The members to add.
     */
    @kotlin.jvm.JvmSynthetic
    @kotlin.jvm.JvmName("addAllMembers")
    fun com.google.protobuf.kotlin.DslList<qaul.net.group.GroupNet.GroupMember, MembersProxy>.addAll(values: kotlin.collections.Iterable<qaul.net.group.GroupNet.GroupMember>) {
      _builder.addAllMembers(values)
    }
    /**
     * <pre>
     * updated members
     * </pre>
     *
     * <code>repeated .qaul.net.group.GroupMember members = 5;</code>
     * @param values The members to add.
     */
    @kotlin.jvm.JvmSynthetic
    @kotlin.jvm.JvmName("plusAssignAllMembers")
    @Suppress("NOTHING_TO_INLINE")
    inline operator fun com.google.protobuf.kotlin.DslList<qaul.net.group.GroupNet.GroupMember, MembersProxy>.plusAssign(values: kotlin.collections.Iterable<qaul.net.group.GroupNet.GroupMember>) {
      addAll(values)
    }
    /**
     * <pre>
     * updated members
     * </pre>
     *
     * <code>repeated .qaul.net.group.GroupMember members = 5;</code>
     * @param index The index to set the value at.
     * @param value The members to set.
     */
    @kotlin.jvm.JvmSynthetic
    @kotlin.jvm.JvmName("setMembers")
    operator fun com.google.protobuf.kotlin.DslList<qaul.net.group.GroupNet.GroupMember, MembersProxy>.set(index: kotlin.Int, value: qaul.net.group.GroupNet.GroupMember) {
      _builder.setMembers(index, value)
    }
    /**
     * <pre>
     * updated members
     * </pre>
     *
     * <code>repeated .qaul.net.group.GroupMember members = 5;</code>
     */
    @kotlin.jvm.JvmSynthetic
    @kotlin.jvm.JvmName("clearMembers")
    fun com.google.protobuf.kotlin.DslList<qaul.net.group.GroupNet.GroupMember, MembersProxy>.clear() {
      _builder.clearMembers()
    }

  }
}
@kotlin.jvm.JvmSynthetic
inline fun qaul.net.group.GroupNet.GroupInfo.copy(block: qaul.net.group.GroupInfoKt.Dsl.() -> kotlin.Unit): qaul.net.group.GroupNet.GroupInfo =
  qaul.net.group.GroupInfoKt.Dsl._create(this.toBuilder()).apply { block() }._build()

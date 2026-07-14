<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import type { CommentTargetType } from '@/types'
import { useAuthStore } from '@/stores/auth'
import { useCommentsStore } from '@/stores/comments'
import { useWorkspacesStore } from '@/stores/workspaces'

const props = defineProps<{
  targetType: CommentTargetType
  targetId: string
}>()

const auth = useAuthStore()
const workspaces = useWorkspacesStore()
const comments = useCommentsStore()
const draft = ref('')

async function load() {
  if (!auth.client || !workspaces.activeWorkspaceId || !props.targetId) return
  await comments.loadForTarget(auth.client, workspaces.activeWorkspaceId, props.targetType, props.targetId)
}

function extractMentions(text: string) {
  return Array.from(text.matchAll(/@([a-zA-Z0-9_-]+)/g), (match) => match[1])
}

async function postComment() {
  if (!auth.client || !auth.user || !workspaces.activeWorkspaceId || !draft.value.trim()) return
  await comments.addComment(auth.client, {
    workspaceId: workspaces.activeWorkspaceId,
    targetType: props.targetType,
    targetId: props.targetId,
    parentCommentId: null,
    body: draft.value.trim(),
    createdBy: auth.user.id,
    mentionedUserIds: extractMentions(draft.value),
  })
  draft.value = ''
  await load()
}

onMounted(load)
watch(() => props.targetId, load)
</script>

<template>
  <aside class="comment-panel">
    <header>
      <strong>留言</strong>
      <span>{{ comments.comments.length }}</span>
    </header>
    <div class="comment-list">
      <article
        v-for="comment in comments.comments"
        :key="comment.id"
        class="comment-item"
      >
        <p>{{ comment.body }}</p>
        <time>{{ comment.createdAt }}</time>
      </article>
      <p
        v-if="comments.comments.length === 0"
        class="comment-empty"
      >
        暂无留言
      </p>
    </div>
    <footer>
      <textarea
        v-model="draft"
        placeholder="写下留言，输入 @成员ID 可提醒"
      />
      <button
        type="button"
        data-action="post-comment"
        :disabled="!draft.trim()"
        @click="postComment"
      >
        发送
      </button>
    </footer>
  </aside>
</template>

<style scoped>
.comment-panel {
  display: grid;
  gap: 8px;
  padding: 10px;
  border-top: 1px solid var(--bb-border);
  background: rgba(7, 17, 25, 0.72);
}

.comment-panel header,
.comment-panel footer {
  display: flex;
  gap: 8px;
  align-items: center;
}

.comment-panel header span {
  color: var(--bb-text-muted);
  font-family: var(--bb-mono);
  font-size: 11px;
}

.comment-list {
  display: grid;
  max-height: 160px;
  overflow: auto;
  gap: 6px;
}

.comment-item {
  padding: 8px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
}

.comment-item p,
.comment-empty {
  margin: 0;
}

.comment-item time {
  color: var(--bb-text-muted);
  font-size: 10px;
}

.comment-panel textarea {
  min-height: 34px;
  flex: 1;
  resize: vertical;
}
</style>

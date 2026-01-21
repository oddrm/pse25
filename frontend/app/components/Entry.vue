<template>
    <tr
        class="cursor-pointer hover:bg-base-300"
        @click="$emit('select', props.entryID)"
    >
        <td>{{ props.name }}</td>
        <td>{{ props.path }}</td>
        <td>{{ props.size }}</td>
        <td>{{ props.platform }}</td>
        <td>
            <span v-for="(tag, index) in props.tags" :key="index" class="badge badge-primary">{{ tag }}</span>
        </td>
    </tr>
</template>

<script setup lang="ts">
import type { Entry, entryID } from '~/utils/entry';

const props = defineProps<Entry>();
defineEmits<{
  (e: 'select', id: entryID): void
}>()

</script>
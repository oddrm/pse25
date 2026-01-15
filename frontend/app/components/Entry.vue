<template>
    <tr>
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
import type { Entry } from "~/utils/entry";
const props = defineProps<Entry>();
</script>
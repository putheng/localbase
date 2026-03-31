<template>
  <div class="flex h-screen bg-slate-950 text-slate-100 overflow-hidden">
    <AppSidebar />
    <div class="flex flex-col flex-1 min-w-0">
      <slot />
    </div>
  </div>
</template>

<script lang="ts">
  import Visualizer from "./Visualizer.svelte";

  interface Props {
    variant?: string;
  }

  let { variant = "bars" }: Props = $props();
</script>

<span class="eq" aria-hidden="true">
  <Visualizer {variant} compact />
</span>

<style>
  .eq {
    display: inline-flex;
    align-items: flex-end;
    width: 24px;
    height: 15px;
    flex: none;
    overflow: hidden;
  }

  :global(:root[data-reduced-motion="true"]) .eq {
    transition: none;
  }
</style>

<script lang="ts">
  interface Props {
    size?: number | string;
    class?: string;
  }

  let { size = 24, class: className = "" }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 256 256"
  fill="none"
  class={className}
  aria-hidden="true"
>
  <path
    d="M 119 21 L 242 21 L 245 23 L 244 28 L 223 55 L 212 67 L 203 74 L 191 80 L 185 82 L 181 82 L 180 83 L 82 83 L 81 84 L 81 131 L 79 138 L 74 147 L 66 155 L 55 160 L 49 160 L 48 161 L 46 160 L 40 160 L 30 156 L 21 148 L 16 140 L 13 131 L 12 116 L 13 115 L 14 106 L 18 97 L 26 87 L 83 31 L 89 27 L 99 23 L 102 23 L 103 22 L 118 22 L 119 21 Z"
    fill="currentColor"
  />
  <path
    d="M 13 230 L 11 228 L 12 225 L 39 193 L 51 182 L 57 178 L 70 172 L 79 171 L 80 170 L 174 170 L 175 169 L 175 121 L 180 109 L 190 100 L 203 95 L 218 96 L 228 101 L 238 112 L 243 127 L 242 146 L 238 156 L 234 162 L 200 197 L 199 197 L 181 216 L 169 226 L 158 230 L 13 230 Z"
    fill="currentColor"
  />
</svg>

module.exports = {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      fontFamily: {
        heading: ['Manrope', 'Inter', 'system-ui', 'sans-serif'],
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace']
      },
      colors: {
        brand: {
          lime: '#e6f18c',
          'lime-hover': '#f0f9a8',
          'lime-soft': '#eef6bd',
          'lime-ink': '#11130d',
          surface: '#0c0d0b',
          'surface-card': '#161814',
          'surface-raised': '#1d201a',
          'surface-input': '#121410',
          'surface-alt': '#242820',
          'surface-glass': 'rgba(22, 24, 20, 0.76)',
          border: '#252921',
          'border-hover': '#373d31',
          'border-strong': '#515a47',
          'border-focus': '#373d31',
          'text-primary': '#f3f5ee',
          'text-secondary': '#9ba395',
          'text-muted': '#687163',
          'text-subtle': '#74796c',
          danger: '#f43f5e',
          'danger-soft': '#3a141c',
          'danger-border': '#572327',
          'danger-text': '#fda4af',
          success: '#10b981',
          'success-soft': '#0f291e',
          'success-border': '#1d442e',
          'success-text': '#6ee7b7',
          warning: '#f59e0b',
          'warning-soft': '#2e2008',
          'warning-border': '#4b3d18',
          'warning-text': '#fcd34d',
          info: '#a3a89a'
        }
      }
    }
  },
  plugins: []
};

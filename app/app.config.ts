export default defineAppConfig({
  ui: {
    colors: {
      primary: 'black',
      success: 'lime',
      warning: 'amber',
      error: 'rose',
      neutral: 'zinc',
    },
    progress: {
      slots: {
        base: 'bg-(--cloudburst-progress-track)',
      },
    },
    input: {
      defaultVariants: {
        variant: 'subtle',
      },
    },
    inputMenu: {
      defaultVariants: {
        variant: 'subtle',
      },
    },
    select: {
      defaultVariants: {
        variant: 'subtle',
      },
    },
    textarea: {
      defaultVariants: {
        variant: 'subtle',
      },
    },
  },
})

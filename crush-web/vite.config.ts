import { defineConfig } from 'vite';
import analog from '@analogjs/platform';
import tsConfigPaths from 'vite-tsconfig-paths';

export default defineConfig(({ mode }) => ({
  ssr: {
    noExternal: ['@spartan-ng/**', '@ng-icons/core', '@ng-icons/lucide', 'clsx'],
  },

  plugins: [
    analog({
      ssr: false,
    }),
    tsConfigPaths(),
  ],
  resolve: {
    alias: {
      '~': '/src/app',
    },
  },
}));

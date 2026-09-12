import nextPlugin from '@next/eslint-plugin-next';
import { defineConfig, globalIgnores } from 'eslint/config';

export default defineConfig([
  globalIgnores(['.next/**', 'dist/**', 'node_modules/**', 'public/**', '*.d.ts']),
  {
    plugins: {
      '@next/next': nextPlugin,
    },
    rules: {
      ...nextPlugin.configs.recommended.rules,
      ...nextPlugin.configs['core-web-vitals'].rules,
      '@next/next/no-img-element': 'off',
    },
  },
]);

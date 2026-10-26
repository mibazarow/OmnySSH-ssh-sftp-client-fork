import type { IconName } from '$lib/theme';

/** One row of an SFTP row's right-click menu. */
export interface MenuItem {
  label: string;
  icon: IconName;
  run: () => void;
  /** Destructive: drawn in the critical colour. */
  danger?: boolean;
}

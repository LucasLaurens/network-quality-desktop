import type { InjectionKey, Ref } from 'vue'
import type { ProfileChoice } from '../bindings'

/** A test the tray menu asked the dashboard to run; consumed once started. */
export const REQUESTED_TEST_KEY: InjectionKey<Ref<ProfileChoice | null>> = Symbol('RequestedTest')

/** The tray menu asked for the help: the dashboard shows it unfolded. */
export const HELP_REQUESTED_KEY: InjectionKey<Ref<boolean>> = Symbol('HelpRequested')

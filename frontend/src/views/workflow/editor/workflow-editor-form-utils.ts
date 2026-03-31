import type { FormField } from '../../../types/task'

export interface EditorFormField {
  key: string
  value: unknown
  type: string
  originalType: string
  description: string
}

/** Accepts task/workflow form definitions (type field may be plain string). */
export function buildFormFields(
  formDef: { key: string; value?: unknown; type?: string; description?: string }[],
): EditorFormField[] {
  return formDef.map(f => ({
    key: f.key,
    value: f.value ?? '',
    type: f.type || 'String',
    originalType: f.type || 'String',
    description: f.description || '',
  }))
}

export function formFieldsToFormArray(fields: EditorFormField[]): FormField[] {
  return fields
    .filter(f => f.key.trim() !== '')
    .map(f => {
      const field: FormField = { key: f.key, value: f.value as FormField['value'], type: f.type as FormField['type'] }
      if (f.description) field.description = f.description
      return field
    })
}

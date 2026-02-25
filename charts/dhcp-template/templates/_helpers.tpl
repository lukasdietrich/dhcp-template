{{/*
Expand the name of the chart.
*/}}
{{- define "dhcp-template.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "dhcp-template.fullname" -}}
{{- if .Values.global.fullnameOverride }}
{{- .Values.global.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.global.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{- define "dhcp-template.operator.fullname" -}}
{{- $name := include "dhcp-template.fullname" . -}}
{{- printf "%s-%s" $name "operator" | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{- define "dhcp-template.agent.fullname" -}}
{{- $name := include "dhcp-template.fullname" . -}}
{{- printf "%s-%s" $name "agent" | trunc 63 | trimSuffix "-" -}}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "dhcp-template.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "dhcp-template.labels" -}}
helm.sh/chart: {{ include "dhcp-template.chart" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{- define "dhcp-template.operator.labels" -}}
{{ include "dhcp-template.labels" . }}
{{ include "dhcp-template.operator.selectorLabels" . }}
{{- end }}

{{- define "dhcp-template.agent.labels" -}}
{{ include "dhcp-template.labels" . }}
{{ include "dhcp-template.agent.selectorLabels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "dhcp-template.selectorLabels" -}}
app.kubernetes.io/name: {{ include "dhcp-template.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "dhcp-template.operator.selectorLabels" -}}
{{ include "dhcp-template.selectorLabels" . }}
app.kubernetes.io/component: operator
{{- end }}

{{- define "dhcp-template.agent.selectorLabels" -}}
{{ include "dhcp-template.selectorLabels" . }}
app.kubernetes.io/component: agent
{{- end }}

{{/*
Create the name of the operator service account to use
*/}}
{{- define "dhcp-template.operator.serviceAccountName" -}}
{{- if .Values.operator.serviceAccount.create }}
{{- default (include "dhcp-template.operator.fullname" .) .Values.operator.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.operator.serviceAccount.name }}
{{- end }}
{{- end }}

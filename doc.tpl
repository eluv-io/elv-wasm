
  
# {{ crate }}
	{%- if license is defined %} ![License: {{ license }}](https://img.shields.io/badge/license-{{ license | replace(from="-", to="--") | urlencode }}-blue){% endif %}
	{%- if crate is defined %} [![{{ crate }} on crates.io](https://img.shields.io/crates/v/{{ crate | urlencode }})](https://crates.io/crates/{{ crate | urlencode }}){% endif %}
	{%- if repository is defined %} [![Source Code Repository](https://img.shields.io/badge/Code-On%20GitHub-blue?logo=GitHub)]({{ repository }}){% endif %}

{{ readme }}

{%- if links != "" %}
{{ links }}
{%- endif -%}


# Scenario template format

The scenario files in this folder need to be kept in a particular format:

* The file must be named `<scenario_name>.html.tmpl` so the HTML generator tool can match a scenario to a template.
* The HTML markup must follow a particular structure because it will eventually get <!-- TODO: remove this to "gets" when this happens --> reused on the holochain.org website, which needs to be able to apply CSS rules predictably. Details about this will follow, but for now you can look at `summary-visualiser/templates/scenarios/*.html.tmpl`.
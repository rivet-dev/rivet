// src/_entries/_icon-component.js
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { createElement } from "react";
function Icon(props) {
  return createElement(FontAwesomeIcon, props);
}
export {
  Icon
};

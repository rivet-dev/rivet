var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleArrowDownLeft.js
var require_faCircleArrowDownLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleArrowDownLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-arrow-down-left";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0f9";
    var svgPathData = "M256 512A256 256 0 1 1 256 0a256 256 0 1 1 0 512zm64-144c13.3 0 24-10.7 24-24s-10.7-24-24-24l-94.1 0L361 185c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0l-135 135L192 184c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 160c0 13.3 10.7 24 24 24l152 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faCircleArrowDownLeft = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faCircleArrowDownLeft.js
var import_faCircleArrowDownLeft = __toESM(require_faCircleArrowDownLeft());
var export_faCircleArrowDownLeft = import_faCircleArrowDownLeft.definition;
export {
  export_faCircleArrowDownLeft as faCircleArrowDownLeft
};

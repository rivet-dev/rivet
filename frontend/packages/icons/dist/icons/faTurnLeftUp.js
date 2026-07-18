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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTurnLeftUp.js
var require_faTurnLeftUp = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTurnLeftUp.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "turn-left-up";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e638";
    var svgPathData = "M34 177.5c-3.8-8.8-2-19 4.6-26l136-144C179.1 2.7 185.4 0 192 0s12.9 2.7 17.4 7.5l136 144c6.6 7 8.4 17.2 4.6 26s-12.5 14.5-22 14.5l-88 0 0 192c0 17.7 14.3 32 32 32l80 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32l-80 0c-70.7 0-128-57.3-128-128l0-192-88 0c-9.6 0-18.2-5.7-22-14.5z";
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
    exports.faTurnLeftUp = exports.definition;
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

// src/_entries/faTurnLeftUp.js
var import_faTurnLeftUp = __toESM(require_faTurnLeftUp());
var export_faTurnLeftUp = import_faTurnLeftUp.definition;
export {
  export_faTurnLeftUp as faTurnLeftUp
};

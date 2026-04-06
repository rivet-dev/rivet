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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPoliceBox.js
var require_faPoliceBox = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPoliceBox.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "police-box";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e021";
    var svgPathData = "M192 0c8.8 0 16 7.2 16 16l0 16 80 0c17.7 0 32 14.3 32 32c17.7 0 32 14.3 32 32l0 368 8 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 512c-13.3 0-24-10.7-24-24s10.7-24 24-24l8 0L32 96c0-17.7 14.3-32 32-32c0-17.7 14.3-32 32-32l80 0 0-16c0-8.8 7.2-16 16-16zM96 136l0 24 32 0 0-32-24 0c-4.4 0-8 3.6-8 8zm0 40l0 24c0 4.4 3.6 8 8 8l24 0 0-32-32 0zm72-48l-24 0 0 32 32 0 0-24c0-4.4-3.6-8-8-8zm-24 48l0 32 24 0c4.4 0 8-3.6 8-8l0-24-32 0zm72-48c-4.4 0-8 3.6-8 8l0 24 32 0 0-32-24 0zm24 48l-32 0 0 24c0 4.4 3.6 8 8 8l24 0 0-32zm16-48l0 32 32 0 0-24c0-4.4-3.6-8-8-8l-24 0zm32 48l-32 0 0 32 24 0c4.4 0 8-3.6 8-8l0-24zM96 256l0 48c0 8.8 7.2 16 16 16l48 0c8.8 0 16-7.2 16-16l0-48c0-8.8-7.2-16-16-16l-48 0c-8.8 0-16 7.2-16 16z";
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
    exports.faPoliceBox = exports.definition;
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

// src/_entries/faPoliceBox.js
var import_faPoliceBox = __toESM(require_faPoliceBox());
var export_faPoliceBox = import_faPoliceBox.definition;
export {
  export_faPoliceBox as faPoliceBox
};

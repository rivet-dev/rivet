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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBusSide.js
var require_faBusSide = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBusSide.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bus-side";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e81d";
    var svgPathData = "M480 0c88.4 0 160 71.6 160 160l0 224c0 35.3-28.7 64-64 64l-5.6 0c-13.2 37.3-48.6 64-90.4 64s-77.3-26.7-90.4-64l-139.1 0c-13.2 37.3-48.7 64-90.4 64s-77.2-26.7-90.4-64L64 448c-35.3 0-64-28.7-64-64L0 96C0 43 43 0 96 0L480 0zM160 368a48 48 0 1 0 0 96 48 48 0 1 0 0-96zm320 0a48 48 0 1 0 0 96 48 48 0 1 0 0-96zm0-304c-17.7 0-32 14.3-32 32l0 192c0 17.7 14.3 32 32 32l64 0c17.7 0 32-14.3 32-32l0-128c0-53-43-96-96-96zM248 224l104 0c17.7 0 32-14.3 32-32l0-96c0-17.7-14.3-32-32-32l-104 0 0 160zM96 64C78.3 64 64 78.3 64 96l0 96c0 17.7 14.3 32 32 32l104 0 0-160-104 0z";
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
    exports.faBusSide = exports.definition;
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

// src/_entries/faBusSide.js
var import_faBusSide = __toESM(require_faBusSide());
var export_faBusSide = import_faBusSide.definition;
export {
  export_faBusSide as faBusSide
};

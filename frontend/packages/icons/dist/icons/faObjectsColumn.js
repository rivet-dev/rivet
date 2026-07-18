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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsColumn.js
var require_faObjectsColumn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsColumn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "objects-column";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e3c1";
    var svgPathData = "M48 32C21.5 32 0 53.5 0 80L0 240c0 26.5 21.5 48 48 48l96 0c26.5 0 48-21.5 48-48l0-160c0-26.5-21.5-48-48-48L48 32zM304 224c-26.5 0-48 21.5-48 48l0 160c0 26.5 21.5 48 48 48l96 0c26.5 0 48-21.5 48-48l0-160c0-26.5-21.5-48-48-48l-96 0zM0 400l0 32c0 26.5 21.5 48 48 48l96 0c26.5 0 48-21.5 48-48l0-32c0-26.5-21.5-48-48-48l-96 0c-26.5 0-48 21.5-48 48zM304 32c-26.5 0-48 21.5-48 48l0 32c0 26.5 21.5 48 48 48l96 0c26.5 0 48-21.5 48-48l0-32c0-26.5-21.5-48-48-48l-96 0z";
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
    exports.faObjectsColumn = exports.definition;
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

// src/_entries/faObjectsColumn.js
var import_faObjectsColumn = __toESM(require_faObjectsColumn());
var export_faObjectsColumn = import_faObjectsColumn.definition;
export {
  export_faObjectsColumn as faObjectsColumn
};

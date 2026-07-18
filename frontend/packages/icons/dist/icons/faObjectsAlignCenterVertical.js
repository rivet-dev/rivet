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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsAlignCenterVertical.js
var require_faObjectsAlignCenterVertical = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsAlignCenterVertical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "objects-align-center-vertical";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3bd";
    var svgPathData = "M64 80c0-26.5 21.5-48 48-48l64 0c26.5 0 48 21.5 48 48l0 152 64 0 0-88c0-26.5 21.5-48 48-48l64 0c26.5 0 48 21.5 48 48l0 88 40 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-40 0 0 88c0 26.5-21.5 48-48 48l-64 0c-26.5 0-48-21.5-48-48l0-88-64 0 0 152c0 26.5-21.5 48-48 48l-64 0c-26.5 0-48-21.5-48-48l0-152-40 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l40 0L64 80z";
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
    exports.faObjectsAlignCenterVertical = exports.definition;
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

// src/_entries/faObjectsAlignCenterVertical.js
var import_faObjectsAlignCenterVertical = __toESM(require_faObjectsAlignCenterVertical());
var export_faObjectsAlignCenterVertical = import_faObjectsAlignCenterVertical.definition;
export {
  export_faObjectsAlignCenterVertical as faObjectsAlignCenterVertical
};

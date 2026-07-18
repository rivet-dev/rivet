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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsAlignLeft.js
var require_faObjectsAlignLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faObjectsAlignLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "objects-align-left";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3be";
    var svgPathData = "M0 24L0 488c0 13.3 10.7 24 24 24s24-10.7 24-24L48 24C48 10.7 37.3 0 24 0S0 10.7 0 24zM176 64c-26.5 0-48 21.5-48 48l0 64c0 26.5 21.5 48 48 48l288 0c26.5 0 48-21.5 48-48l0-64c0-26.5-21.5-48-48-48L176 64zm0 224c-26.5 0-48 21.5-48 48l0 64c0 26.5 21.5 48 48 48l160 0c26.5 0 48-21.5 48-48l0-64c0-26.5-21.5-48-48-48l-160 0z";
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
    exports.faObjectsAlignLeft = exports.definition;
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

// src/_entries/faObjectsAlignLeft.js
var import_faObjectsAlignLeft = __toESM(require_faObjectsAlignLeft());
var export_faObjectsAlignLeft = import_faObjectsAlignLeft.definition;
export {
  export_faObjectsAlignLeft as faObjectsAlignLeft
};

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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkSimple.js
var require_faLinkSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLinkSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "link-simple";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e1cd";
    var svgPathData = "M0 256C0 167.6 71.6 96 160 96l64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-64 0c-53 0-96 43-96 96s43 96 96 96l64 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-64 0C71.6 416 0 344.4 0 256zm576 0c0 88.4-71.6 160-160 160l-64 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l64 0c53 0 96-43 96-96s-43-96-96-96l-64 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l64 0c88.4 0 160 71.6 160 160zM192 224l192 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-192 0c-17.7 0-32-14.3-32-32s14.3-32 32-32z";
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
    exports.faLinkSimple = exports.definition;
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

// src/_entries/faLinkSimple.js
var import_faLinkSimple = __toESM(require_faLinkSimple());
var export_faLinkSimple = import_faLinkSimple.definition;
export {
  export_faLinkSimple as faLinkSimple
};

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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaceFlushed.js
var require_faFaceFlushed = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaceFlushed.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-flushed";
    var width = 512;
    var height = 512;
    var aliases = [128563, "flushed"];
    var unicode = "f579";
    var svgPathData = "M0 256a256 256 0 1 1 512 0 256 256 0 1 1 -512 0zM168 392c0 13.3 10.7 24 24 24l128 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-128 0c-13.3 0-24 10.7-24 24zm-8-96a72 72 0 1 0 0-144 72 72 0 1 0 0 144zm264-72a72 72 0 1 0 -144 0 72 72 0 1 0 144 0zm-288 0a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm192 0a24 24 0 1 1 48 0 24 24 0 1 1 -48 0z";
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
    exports.faFaceFlushed = exports.definition;
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

// src/_entries/faFlushed.js
var import_faFaceFlushed = __toESM(require_faFaceFlushed());
var export_faFlushed = import_faFaceFlushed.definition;
export {
  export_faFlushed as faFlushed
};

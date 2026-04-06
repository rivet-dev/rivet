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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUserTag.js
var require_faUserTag = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUserTag.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-tag";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f507";
    var svgPathData = "M256.1 8a120 120 0 1 1 0 240 120 120 0 1 1 0-240zM226.4 304l59.4 0c6.7 0 13.2 .4 19.7 1.1-.9 4.9-1.4 9.9-1.4 15l0 92.1c0 25.5 10.1 49.9 28.1 67.9l31.9 31.9-286.3 0c-16.4 0-29.7-13.3-29.7-29.7 0-98.5 79.8-178.3 178.3-178.3zM352.1 412.2l0-92.1c0-17.7 14.3-32 32-32l92.1 0c12.7 0 24.9 5.1 33.9 14.1l96 96c18.7 18.7 18.7 49.1 0 67.9l-76.1 76.1c-18.7 18.7-49.1 18.7-67.9 0l-96-96c-9-9-14.1-21.2-14.1-33.9zm104-44.2a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faUserTag = exports.definition;
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

// src/_entries/faUserTag.js
var import_faUserTag = __toESM(require_faUserTag());
var export_faUserTag = import_faUserTag.definition;
export {
  export_faUserTag as faUserTag
};

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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTvRetro.js
var require_faTvRetro = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTvRetro.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tv-retro";
    var width = 512;
    var height = 512;
    var aliases = [128250];
    var unicode = "f401";
    var svgPathData = "M169 7c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l55 55L80 96C35.8 96 0 131.8 0 176L0 432c0 44.2 35.8 80 80 80l352 0c44.2 0 80-35.8 80-80l0-256c0-44.2-35.8-80-80-80L321.9 96l55-55c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0l-87 87L169 7zM424 232a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm24 56a24 24 0 1 1 0 48 24 24 0 1 1 0-48zM64 224c0-35.3 28.7-64 64-64l192 0c35.3 0 64 28.7 64 64l0 160c0 35.3-28.7 64-64 64l-192 0c-35.3 0-64-28.7-64-64l0-160z";
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
    exports.faTvRetro = exports.definition;
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

// src/_entries/faTvRetro.js
var import_faTvRetro = __toESM(require_faTvRetro());
var export_faTvRetro = import_faTvRetro.definition;
export {
  export_faTvRetro as faTvRetro
};

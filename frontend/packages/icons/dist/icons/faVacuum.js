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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faVacuum.js
var require_faVacuum = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faVacuum.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "vacuum";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e04d";
    var svgPathData = "M301.6 64L512 64c35.3 0 64 28.7 64 64c0 21.9-11 41.2-27.8 52.8C513.8 148.1 467.2 128 416 128c-35.3 0-64 28.7-64 64l0 100c10.2-2.6 21-4 32-4c70.7 0 128 57.3 128 128c0 38.2-16.8 72.5-43.3 96l75.3 0c35.3 0 64-28.7 64-64l0-128c0-31.7-7.7-61.7-21.3-88C619 208.7 640 170.8 640 128C640 57.3 582.7 0 512 0L301.6 0C241.3 0 189.1 42.2 176.4 101.2L150.1 224l65.5 0L239 114.6C245.4 85.1 271.4 64 301.6 64zM208.7 256l-65.5 0L109 416l-25.2 0c-33.8 0-64.7 18.9-80 49C1.4 469.5 0 474.6 0 480c0 17.7 14.3 32 32 32l95.4 0c.4 0 .9 0 1.3 0l47.3 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-1.6 0 34.3-160zM480 416a96 96 0 1 0 -192 0 96 96 0 1 0 192 0zm-128 0a32 32 0 1 1 64 0 32 32 0 1 1 -64 0z";
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
    exports.faVacuum = exports.definition;
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

// src/_entries/faVacuum.js
var import_faVacuum = __toESM(require_faVacuum());
var export_faVacuum = import_faVacuum.definition;
export {
  export_faVacuum as faVacuum
};

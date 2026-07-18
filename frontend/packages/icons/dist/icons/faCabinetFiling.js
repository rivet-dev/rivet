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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCabinetFiling.js
var require_faCabinetFiling = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCabinetFiling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cabinet-filing";
    var width = 448;
    var height = 512;
    var aliases = [128452];
    var unicode = "f64b";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L384 0c35.3 0 64 28.7 64 64l0 176L0 240 0 64zM0 272l448 0 0 176c0 35.3-28.7 64-64 64L64 512c-35.3 0-64-28.7-64-64L0 272zM128 128l0 16c0 8.8 7.2 16 16 16s16-7.2 16-16l0-16 128 0 0 16c0 8.8 7.2 16 16 16s16-7.2 16-16l0-16c0-17.7-14.3-32-32-32L160 96c-17.7 0-32 14.3-32 32zm0 256l0 16c0 8.8 7.2 16 16 16s16-7.2 16-16l0-16 128 0 0 16c0 8.8 7.2 16 16 16s16-7.2 16-16l0-16c0-17.7-14.3-32-32-32l-128 0c-17.7 0-32 14.3-32 32z";
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
    exports.faCabinetFiling = exports.definition;
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

// src/_entries/faCabinetFiling.js
var import_faCabinetFiling = __toESM(require_faCabinetFiling());
var export_faCabinetFiling = import_faCabinetFiling.definition;
export {
  export_faCabinetFiling as faCabinetFiling
};

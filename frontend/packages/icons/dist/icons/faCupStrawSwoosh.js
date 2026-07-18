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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCupStrawSwoosh.js
var require_faCupStrawSwoosh = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCupStrawSwoosh.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cup-straw-swoosh";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e364";
    var svgPathData = "M194.8 43.2l-20 84.8L32 128c-17.7 0-32 14.3-32 32s14.3 32 32 32l320 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-127.8 0 17.4-73.8c.9-3.6 4.1-6.2 7.8-6.2L280 48c13.3 0 24-10.7 24-24s-10.7-24-24-24L249.3 0c-26 0-48.6 17.9-54.5 43.2zM40 224L60.3 468c2.1 24.9 22.9 44 47.8 44l167.7 0c25 0 45.8-19.1 47.8-44L344 224 40 224zm194.4 78.9c18.8-10.1 42-17.2 72.7-20.6L303 331.2c-20.8 3.1-35.2 8.2-45.8 13.9c-17.2 9.3-26.9 21.2-39.9 37.2l-.7 .8c-13.3 16.4-29.5 35.9-58.7 50.4c-17.7 8.8-39.3 15.2-66.7 18.9l-4-47.9c22.1-3.2 37.7-8.3 49.4-14.1c19.3-9.5 30.1-22 42.8-37.6l1.9-2.3c12.3-15.2 27.4-33.8 53.1-47.7z";
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
    exports.faCupStrawSwoosh = exports.definition;
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

// src/_entries/faCupStrawSwoosh.js
var import_faCupStrawSwoosh = __toESM(require_faCupStrawSwoosh());
var export_faCupStrawSwoosh = import_faCupStrawSwoosh.definition;
export {
  export_faCupStrawSwoosh as faCupStrawSwoosh
};

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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBridgeSuspension.js
var require_faBridgeSuspension = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBridgeSuspension.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bridge-suspension";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e4cd";
    var svgPathData = "M128 8c9.1 0 17 5 21.1 12.5C184.2 81.6 249.7 120 320 120s135.8-38.4 170.9-99.5C495 13 502.9 8 512 8c8.8 0 16.5 4.7 20.7 11.8c13.1 21.2 26.6 42.2 42 59.8c15.5 17.7 31.7 30.6 49.4 36.9c12.5 4.5 18.9 18.2 14.4 30.7s-18.2 18.9-30.7 14.4c-25.5-9.2-46.4-25.8-63.9-44.5L544 256l64 0c17.7 0 32 14.3 32 32l0 32c0 17.7-14.3 32-32 32c-35.3 0-64 28.7-64 64l0 64c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-32c0-53-43-96-96-96l-64 0c-53 0-96 43-96 96l0 32c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-64c0-35.3-28.7-64-64-64c-17.7 0-32-14.3-32-32l0-32c0-17.7 14.3-32 32-32l64 0 0-138.9c-17.4 18.7-38.4 35.3-63.9 44.5c-12.5 4.5-26.2-2-30.7-14.4s2-26.2 14.4-30.7c17.7-6.4 33.9-19.3 49.4-36.9c15.4-17.6 29-38.6 42-59.8C111.5 12.7 119.2 8 128 8zM480 108.1c-12.4 10.7-25.8 20.1-40 28.2L440 256l40 0 0-147.9zm-88 49c-15.5 4.8-31.6 8.1-48 9.8l0 89.2 48 0 0-99zm-96 9.8c-16.4-1.6-32.5-4.9-48-9.8l0 99 48 0 0-89.2zm-96-30.5c-14.2-8-27.6-17.5-40-28.2L160 256l40 0 0-119.7z";
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
    exports.faBridgeSuspension = exports.definition;
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

// src/_entries/faBridgeSuspension.js
var import_faBridgeSuspension = __toESM(require_faBridgeSuspension());
var export_faBridgeSuspension = import_faBridgeSuspension.definition;
export {
  export_faBridgeSuspension as faBridgeSuspension
};

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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMugMarshmallows.js
var require_faMugMarshmallows = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMugMarshmallows.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mug-marshmallows";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7b7";
    var svgPathData = "M192 32c7.5 0 14.4 2.6 19.8 6.9L175 75.7c-9.6 9.6-15 22.7-15 36.3c0 5.5 .9 10.9 2.6 16L32 128l0-64c0-17.7 14.3-32 32-32l128 0zm5.7 93.7C194 122 192 117.1 192 112s2-10 5.7-13.7l60.7-60.7C262 34 266.9 32 272 32s10 2 13.7 5.7l60.7 60.7c3.6 3.6 5.7 8.5 5.7 13.7s-2 10-5.7 13.7L344 128l-144 0-2.3-2.3zM0 192c0-17.7 14.3-32 32-32l64 0 0 64c0 17.7 14.3 32 32 32s32-14.3 32-32l0-64 192 0 48 0c61.9 0 112 50.1 112 112s-50.1 112-112 112l-16 0c0 53-43 96-96 96L96 480c-53 0-96-43-96-96L0 192zm384 32l0 96 16 0c26.5 0 48-21.5 48-48s-21.5-48-48-48l-16 0z";
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
    exports.faMugMarshmallows = exports.definition;
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

// src/_entries/faMugMarshmallows.js
var import_faMugMarshmallows = __toESM(require_faMugMarshmallows());
var export_faMugMarshmallows = import_faMugMarshmallows.definition;
export {
  export_faMugMarshmallows as faMugMarshmallows
};

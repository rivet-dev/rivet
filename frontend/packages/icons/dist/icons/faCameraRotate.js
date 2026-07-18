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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCameraRotate.js
var require_faCameraRotate = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCameraRotate.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-rotate";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0d8";
    var svgPathData = "M138.7 96l10.4-31.2C155.6 45.2 173.9 32 194.6 32l122.8 0c20.7 0 39 13.2 45.5 32.8L373.3 96 448 96c35.3 0 64 28.7 64 64l0 256c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 160c0-35.3 28.7-64 64-64l74.7 0zM96 305.9c0 3.9 1.4 7.7 4 10.6l48 54c6.4 7.2 17.6 7.2 23.9 0l48-54c2.6-2.9 4-6.7 4-10.6l0-1.9c0-8.8-7.2-16-16-16l-24 0c0-39.8 32.2-72 72-72 4.4 0 8.7 .4 12.9 1.2l34.9-39.3c-14.6-6.4-30.8-9.9-47.8-9.9-66.3 0-120 53.7-120 120l-24 0c-8.8 0-16 7.2-16 16l0 1.9zm196-46.5c-2.6 2.9-4 6.7-4 10.6l0 1.9c0 8.8 7.2 16 16 16l24 0c0 39.8-32.2 72-72 72-4.4 0-8.7-.4-12.9-1.2l-34.9 39.3c14.7 6.4 30.8 9.9 47.8 9.9 66.3 0 120-53.7 120-120l24 0c8.8 0 16-7.2 16-16l0-1.9c0-3.9-1.4-7.7-4-10.6l-48-54c-6.4-7.2-17.6-7.2-23.9 0l-48 54z";
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
    exports.faCameraRotate = exports.definition;
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

// src/_entries/faCameraRotate.js
var import_faCameraRotate = __toESM(require_faCameraRotate());
var export_faCameraRotate = import_faCameraRotate.definition;
export {
  export_faCameraRotate as faCameraRotate
};

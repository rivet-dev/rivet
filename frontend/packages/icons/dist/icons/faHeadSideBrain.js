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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHeadSideBrain.js
var require_faHeadSideBrain = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHeadSideBrain.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "head-side-brain";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f808";
    var svgPathData = "M0 224.2C0 100.6 100.2 0 224 0l24 0c95.2 0 181.2 69.3 197.3 160.2c2.3 13 6.8 25.7 15.1 36l42 52.6c6.2 7.8 9.6 17.4 9.6 27.4c0 24.2-19.6 43.8-43.8 43.8L448 320l0 64c0 35.3-28.7 64-64 64l-64 0 0 32c0 17.7-14.3 32-32 32L96 512c-17.7 0-32-14.3-32-32l0-72.7c0-16.7-6.9-32.5-17.1-45.8C16.6 322.4 0 274.1 0 224.2zM285.3 208l50.7 0c26.5 0 48-21.5 48-48s-21.5-48-48-48c-.9 0-1.8 0-2.7 .1C326.7 93.4 308.9 80 288 80c-8.6 0-16.6 2.2-23.5 6.2C255.9 72.8 241 64 224 64s-31.9 8.8-40.5 22.2c-7-3.9-15-6.2-23.5-6.2c-26.5 0-48 21.5-48 48c-26.5 0-48 21.5-48 48c0 20.9 13.4 38.7 32.1 45.3c0 .9-.1 1.8-.1 2.7c0 26.5 21.5 48 48 48c5.6 0 11-1 16-2.7l0 18.7c0 17.7 14.3 32 32 32s32-14.3 32-32l0-18.7c5 1.8 10.4 2.7 16 2.7c26.5 0 48-21.5 48-48c0-5.6-1-11-2.7-16zM160 176l0 2.7c0 0 0 0-.1 0c0-.9 .1-1.8 .1-2.7z";
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
    exports.faHeadSideBrain = exports.definition;
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

// src/_entries/faHeadSideBrain.js
var import_faHeadSideBrain = __toESM(require_faHeadSideBrain());
var export_faHeadSideBrain = import_faHeadSideBrain.definition;
export {
  export_faHeadSideBrain as faHeadSideBrain
};

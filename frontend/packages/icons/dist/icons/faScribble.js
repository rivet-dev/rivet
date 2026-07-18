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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScribble.js
var require_faScribble = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScribble.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scribble";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e23f";
    var svgPathData = "M170 23.3C186.5 8.3 208 0 230.3 0l1.1 0C280.3 0 320 39.7 320 88.6c0 23.5-9.3 46-25.9 62.6L103.2 342.1c-4.6 4.6-7.2 10.9-7.2 17.4C96 373 107 384 120.6 384c6.5 0 12.8-2.6 17.4-7.2L360.8 153.9c16.6-16.6 39.1-25.9 62.6-25.9c48.9 0 88.6 39.7 88.6 88.6c0 23.5-9.3 46-25.9 62.6l-88.2 88.2c-8.8 8.8-13.8 20.9-13.8 33.4c0 26 21.1 47.1 47 47.1c6.2 0 12.4-1.5 18-4.2l16.7-8.4c15.8-7.9 35-1.5 42.9 14.3s1.5 35-14.3 42.9L477.6 501c-14.5 7.2-30.4 11-46.6 11c-61.4 0-111-49.9-111-111.1c0-29.5 11.7-57.8 32.6-78.7l88.2-88.2c4.6-4.6 7.2-10.9 7.2-17.4c0-13.6-11-24.6-24.6-24.6c-6.5 0-12.8 2.6-17.4 7.2L183.2 422.1c-16.6 16.6-39.1 25.9-62.6 25.9C71.7 448 32 408.3 32 359.4c0-23.5 9.3-46 25.9-62.6L248.8 105.9c4.6-4.6 7.2-10.9 7.2-17.4C256 75 245 64 231.4 64l-1.1 0c-6.4 0-12.6 2.4-17.3 6.7L53.5 215.7c-13.1 11.9-33.3 10.9-45.2-2.2s-10.9-33.3 2.2-45.2L170 23.3z";
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
    exports.faScribble = exports.definition;
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

// src/_entries/faScribble.js
var import_faScribble = __toESM(require_faScribble());
var export_faScribble = import_faScribble.definition;
export {
  export_faScribble as faScribble
};

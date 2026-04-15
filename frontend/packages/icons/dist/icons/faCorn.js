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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCorn.js
var require_faCorn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCorn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "corn";
    var width = 512;
    var height = 512;
    var aliases = [127805];
    var unicode = "f6c7";
    var svgPathData = "M512 40c0 15-8.3 28.1-20.5 34.9C499.2 82.2 504 92.6 504 104c0 18-11.8 33.2-28.2 38.2c2.7 5.4 4.2 11.4 4.2 17.8c0 18-11.8 33.2-28.2 38.2c2.7 5.4 4.2 11.4 4.2 17.8c0 22.1-17.9 40-40 40c-1.1 0-2.3 0-3.4-.1c2.2 4.9 3.4 10.4 3.4 16.1c0 13.6-6.8 25.7-17.3 32.9c-47.7-29.7-103.6-41.1-157.5-34.2c6.9-53.9-4.5-109.8-34.2-157.5C214.3 102.8 226.4 96 240 96c5.7 0 11.2 1.2 16.1 3.4c-.1-1.1-.1-2.2-.1-3.4c0-22.1 17.9-40 40-40c6.4 0 12.4 1.5 17.8 4.2C318.8 43.8 334 32 352 32c6.4 0 12.4 1.5 17.8 4.2C374.8 19.8 390 8 408 8c11.4 0 21.8 4.8 29.1 12.5C443.9 8.3 457 0 472 0c22.1 0 40 17.9 40 40zM137.7 98c1.2-6.3 8.9-8.7 13.2-3.9c45.4 50.5 69.5 114 56.9 183.4c-38.7 10.7-75.2 31.2-105.5 61.6L24.9 416.5c-34.2-37.7-33.1-96 3.2-132.4l64-64c18.3-18.3 30.6-41.7 35.3-67.1L137.7 98zM92.1 483.9L47.4 439.2l77.5-77.5c28.6-28.6 63.5-47.1 100.1-55.6c72.2-16.6 139.8 7.4 192.8 55c4.8 4.3 2.4 12.1-3.9 13.2l-54.9 10.2c-25.4 4.7-48.8 17-67.1 35.3l-64 64c-37.5 37.5-98.3 37.5-135.8 0z";
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
    exports.faCorn = exports.definition;
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

// src/_entries/faCorn.js
var import_faCorn = __toESM(require_faCorn());
var export_faCorn = import_faCorn.definition;
export {
  export_faCorn as faCorn
};

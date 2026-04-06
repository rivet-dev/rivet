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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRouteHighway.js
var require_faRouteHighway = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRouteHighway.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "route-highway";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f61a";
    var svgPathData = "M240.5 4.6c-10.1-6.1-22.8-6.1-32.9 0l-4.7 2.8c-37.5 22.5-84 24.3-123.1 4.7l-1.4-.7c-15.2-7.6-33.7-2-42.1 12.7l-32 56c-5.3 9.3-5.6 20.6-.8 30.2l16.2 32.5C27.8 159.1 32 177 32 195.2c0 14.9-2.8 29.7-8.4 43.6L9.3 274.5C3.2 289.9 0 306.4 0 323c0 49.5 28 94.8 72.2 116.9l137.4 68.7c9 4.5 19.6 4.5 28.6 0l137.4-68.7C420 417.8 448 372.5 448 323c0-16.6-3.2-33.1-9.3-48.5l-14.3-35.7c-5.5-13.9-8.4-28.6-8.4-43.6c0-18.2 4.2-36.2 12.4-52.5l16.2-32.5c4.8-9.6 4.5-20.9-.8-30.2l-32-56c-8.4-14.7-26.9-20.3-42.1-12.7l-1.4 .7c-39.1 19.6-85.6 17.8-123.1-4.7l-4.7-2.8zM96 192L352 192c0 1.1 0 2.2 0 3.2c0 23.1 4.4 45.9 13 67.3l14.3 35.7c3.2 7.9 4.8 16.3 4.8 24.8c0 25.3-14.3 48.4-36.9 59.7L224 444.2 100.9 382.7C78.3 371.4 64 348.3 64 323c0-8.5 1.6-16.9 4.8-24.8L83 262.6c8.6-21.4 13-44.3 13-67.3c0-1.1 0-2.2 0-3.2z";
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
    exports.faRouteHighway = exports.definition;
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

// src/_entries/faRouteHighway.js
var import_faRouteHighway = __toESM(require_faRouteHighway());
var export_faRouteHighway = import_faRouteHighway.definition;
export {
  export_faRouteHighway as faRouteHighway
};

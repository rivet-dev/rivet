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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGlobePointer.js
var require_faGlobePointer = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGlobePointer.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "globe-pointer";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e60e";
    var svgPathData = "M352 256c0 22.2-1.2 43.6-3.3 64l-66.8 0 12.6-42.8c10.7-36.4-23.1-70.3-59.6-59.6l-74.6 21.9c.4-16.3 1.5-32.2 3.1-47.5l185.3 0c2.2 20.4 3.3 41.8 3.3 64zM20.4 280.6c-7.1 2.1-13.1 5.5-18.1 9.9C.8 279.2 0 267.7 0 256c0-22.1 2.8-43.5 8.1-64l123.1 0c-1.9 18.4-2.9 37.4-3.1 57L20.4 280.6zM231.4 491.6L272.4 352l71.9 0c-6.1 36.4-15.5 68.6-27 94.6c-10.5 23.6-22.2 40.7-33.5 51.5C272.6 508.8 263.3 512 256 512c-7.2 0-16.3-3.1-27.3-13.4c1-2.2 1.9-4.6 2.7-7.1zM380.8 192l123.1 0c5.3 20.5 8.1 41.9 8.1 64s-2.8 43.5-8.1 64l-123.1 0c2.1-20.6 3.2-42 3.2-64s-1.1-43.4-3.2-64zm112.6-32l-116.7 0c-10-63.9-29.8-117.4-55.3-151.6c78.3 20.7 142 77.5 171.9 151.6zm-325.7 0c6.1-36.4 15.5-68.6 27-94.7c10.5-23.6 22.2-40.7 33.5-51.5C239.4 3.2 248.7 0 256 0s16.6 3.2 27.8 13.8c11.3 10.8 23 27.9 33.5 51.5c11.6 26 20.9 58.2 27 94.7l-176.6 0zm-32.4 0L18.6 160C48.6 85.9 112.2 29.1 190.6 8.4C165.1 42.6 145.3 96.1 135.3 160zM493.4 352c-30 74.1-93.6 130.9-171.9 151.6c25.5-34.2 45.2-87.7 55.3-151.6l116.7 0zM39 308.5l204.8-60.2c12.1-3.6 23.4 7.7 19.9 19.9L203.5 473c-4.1 13.9-23.2 15.6-29.7 2.6l-28.7-57.3c-.7-1.3-1.5-2.6-2.5-3.7l-88 88c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3l88-88c-1.1-1-2.3-1.9-3.7-2.5L36.4 338.2c-13-6.5-11.3-25.6 2.6-29.7z";
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
    exports.faGlobePointer = exports.definition;
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

// src/_entries/faGlobePointer.js
var import_faGlobePointer = __toESM(require_faGlobePointer());
var export_faGlobePointer = import_faGlobePointer.definition;
export {
  export_faGlobePointer as faGlobePointer
};

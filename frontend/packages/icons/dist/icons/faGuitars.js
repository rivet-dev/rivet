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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGuitars.js
var require_faGuitars = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGuitars.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "guitars";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f8bf";
    var svgPathData = "M104 0l48 0c13.3 0 24 10.7 24 24l0 52.2c0 6.2-1.4 12.3-4.2 17.9L152 133.7l0 44.4c50.3 8.6 88 43.8 88 86c0 14-4.2 27.3-11.6 39.1c-8.2 13-7.5 32 2.9 43.4c5.9 6.5 10.9 13.7 14.8 21.3c-2.3 4.9-4.7 9.6-7 13.9c-8.8 16.3-15 35.6-15 56.5c0 11.6 1.9 22.8 5.4 33.3C205.9 496.1 169.3 512 128 512C57.3 512 0 465.4 0 408c0-23 9.2-44.3 24.8-61.5c10.3-11.4 11.1-30.4 2.9-43.4C20.2 291.3 16 278 16 264c0-42.1 37.7-77.3 88-86l0-44.4L84.2 94.1C81.4 88.6 80 82.4 80 76.2L80 24C80 10.7 90.7 0 104 0zm24 344a40 40 0 1 0 0-80 40 40 0 1 0 0 80zM360 148.6l-21.4-24.5c-6.6-7.6-9.3-17.9-7.2-27.8l13.9-64.8C349.2 13.1 365.4 0 384.2 0c22 0 39.8 17.8 39.8 39.8l0 4.6c0 12.4-6.4 23.3-16 29.7l0 164.7c27.9-4.2 39.6-19.4 44.5-31.3c3.3-8.2 10.7-15.5 19.5-15.5s16 7.2 16 16l0 29.3c0 11.6-4.8 22.8-9.3 33.5l-.5 1.1c-3.3 7.8-6.2 18.5-6.2 32.1c0 33.5 17 70.9 28.8 92.9c6.9 12.7 11.2 26.9 11.2 41.3c0 40.8-33 73.8-73.8 73.8l-108.4 0C289 512 256 479 256 438.2c0-14.5 4.3-28.6 11.2-41.3C279 374.9 296 337.5 296 304c0-11.2-2-20.5-4.5-27.8c-.9-2.5-1.8-5-2.7-7.5c0 0 0 0 0 0s0 0 0 0c-4.3-11.8-8.8-23.8-8.8-36.2l0-40.6c0-8.8 7.2-16 16-16s16.1 7.2 18.2 15.8c3.9 16.4 14.9 40.5 45.8 46.7l0-89.9zM336 320c0 8.8 7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0c-8.8 0-16 7.2-16 16zm0 64c0 8.8 7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0c-8.8 0-16 7.2-16 16z";
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
    exports.faGuitars = exports.definition;
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

// src/_entries/faGuitars.js
var import_faGuitars = __toESM(require_faGuitars());
var export_faGuitars = import_faGuitars.definition;
export {
  export_faGuitars as faGuitars
};

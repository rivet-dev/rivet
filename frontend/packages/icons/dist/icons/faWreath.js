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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWreath.js
var require_faWreath = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWreath.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wreath";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f7e2";
    var svgPathData = "M182.6 0c15.8 0 30.2 5.7 41.4 15.2C235.2 5.7 249.6 0 265.4 0c27.9 0 51.6 17.8 60.3 42.6c5.8-1.7 11.9-2.6 18.2-2.6c35.3 0 64 28.7 64 64c0 6.3-.9 12.5-2.6 18.2c24.8 8.8 42.6 32.5 42.6 60.3c0 15.8-5.7 30.2-15.2 41.4c9.5 11.2 15.2 25.6 15.2 41.4c0 27.9-17.8 51.6-42.6 60.3c1.7 5.8 2.6 11.9 2.6 18.2c0 32.6-24.4 59.6-56 63.5l0-3.7c0-28.6-23.2-51.8-51.8-51.8c-7.2 0-14.4 1.5-21 4.5L224 381l-55.2-24.5c-6.6-2.9-13.8-4.5-21-4.5C119.2 352 96 375.2 96 403.8l0 3.7c-31.6-3.9-56-30.9-56-63.5c0-6.3 .9-12.5 2.6-18.2C17.8 317 0 293.3 0 265.4c0-15.8 5.7-30.2 15.2-41.4C5.7 212.8 0 198.4 0 182.6c0-27.9 17.8-51.6 42.6-60.3C40.9 116.5 40 110.3 40 104c0-35.3 28.7-64 64-64c6.3 0 12.5 .9 18.2 2.6C131 17.8 154.7 0 182.6 0zM203 142.4c-5.7 6-13.6 9.4-21.9 9.1c-16.6-.6-30.2 13-29.6 29.6c.3 8.3-3 16.3-9.1 21.9c-12.1 11.3-12.1 30.6 0 41.9c6 5.7 9.4 13.6 9.1 21.9c-.6 16.6 13 30.2 29.6 29.6c8.3-.3 16.3 3 21.9 9.1c11.3 12.1 30.6 12.1 41.9 0c5.7-6 13.6-9.4 21.9-9.1c16.6 .6 30.2-13 29.6-29.6c-.3-8.3 3-16.3 9.1-21.9c12.1-11.3 12.1-30.6 0-41.9c-6-5.7-9.4-13.6-9.1-21.9c.6-16.6-13-30.2-29.6-29.6c-8.3 .3-16.3-3-21.9-9.1c-11.3-12.1-30.6-12.1-41.9 0zM128 403.8c0-10.9 8.9-19.8 19.8-19.8c2.8 0 5.5 .6 8 1.7L224 416l68.2-30.3c2.5-1.1 5.3-1.7 8-1.7c10.9 0 19.8 8.9 19.8 19.8l0 88.4c0 10.9-8.9 19.8-19.8 19.8c-2.8 0-5.5-.6-8-1.7L224 480l-68.2 30.3c-2.5 1.1-5.3 1.7-8 1.7c-10.9 0-19.8-8.9-19.8-19.8l0-88.4z";
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
    exports.faWreath = exports.definition;
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

// src/_entries/faWreath.js
var import_faWreath = __toESM(require_faWreath());
var export_faWreath = import_faWreath.definition;
export {
  export_faWreath as faWreath
};

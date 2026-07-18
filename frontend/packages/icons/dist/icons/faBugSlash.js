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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBugSlash.js
var require_faBugSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBugSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bug-slash";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e490";
    var svgPathData = "M7-24.9c9.4-9.4 24.6-9.4 33.9 0L241.8 176 344 176c14.2 0 27.7 2.8 40 8l108.8-81.6c14.1-10.6 34.2-7.7 44.8 6.4s7.7 34.2-6.4 44.8l-97.8 73.3c5.3 8.9 9.3 18.7 11.8 29.1l98.8 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-96 0 0 32c0 9.5-.8 18.7-2.4 27.8L569 503.1c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0L7 9.1C-2.3-.3-2.3-15.5 7-24.9zM398.2 468c-23.2 22.1-53 37.2-86.2 42.2l0-128.4 86.2 86.2zM264 333.8l0 176.4c-51.2-7.7-94.5-39.7-117.7-83.9L83.2 473.6c-14.1 10.6-34.2 7.7-44.8-6.4s-7.7-34.2 6.4-44.8l83.4-62.5c-.1-2.6-.2-5.2-.2-7.9l0-32-96 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l98.8 0c3.3-13.9 9.3-26.7 17.6-37.8L264 333.8zM355.6 128l-82.4 0-72.3-72.3C216.1 22.8 249.4 0 288 0 341 0 384 43 384 96l0 3.6c0 15.7-12.7 28.4-28.4 28.4z";
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
    exports.faBugSlash = exports.definition;
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

// src/_entries/faBugSlash.js
var import_faBugSlash = __toESM(require_faBugSlash());
var export_faBugSlash = import_faBugSlash.definition;
export {
  export_faBugSlash as faBugSlash
};

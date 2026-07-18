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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPenNibSlash.js
var require_faPenNibSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPenNibSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pen-nib-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e4a1";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L450.3 327.6l28.3-94.4 2.8-9.2L352 94.6l-9.2 2.8L208.1 137.8 38.8 5.1zm29 440.7c-3.8 11.3-1 23.9 7.3 32.4L228.7 324.7c-3-6.3-4.7-13.3-4.7-20.7c0-9.4 2.7-18.2 7.4-25.7l-85-67L67.8 445.8zM272 352c-7.4 0-14.4-1.7-20.7-4.7L97.7 500.9c8.6 8.3 21.1 11.2 32.4 7.3l264.3-88.6c3.6-1.2 7.1-2.8 10.5-4.6l-98.5-77.6c-8.7 9-20.9 14.6-34.4 14.6zM432.4 18.3L376.7 74.1 501.9 199.3l55.7-55.7c21.9-21.9 21.9-57.3 0-79.2L511.6 18.3c-21.9-21.9-57.3-21.9-79.2 0z";
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
    exports.faPenNibSlash = exports.definition;
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

// src/_entries/faPenNibSlash.js
var import_faPenNibSlash = __toESM(require_faPenNibSlash());
var export_faPenNibSlash = import_faPenNibSlash.definition;
export {
  export_faPenNibSlash as faPenNibSlash
};

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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPlaneUpSlash.js
var require_faPlaneUpSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPlaneUpSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "plane-up-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e22e";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L482.4 352.8 554.9 377c10.4 3.5 21.1-4.3 21.1-15.2l0-56.7c0-10.7-5.3-20.7-14.2-26.6L384 160l0-66.3C384 59.5 356 0 320 0c-35 0-64 59.5-64 93.7l0 66.3-10.6 7.1L38.8 5.1zm122.5 218L78.2 278.5C69.3 284.4 64 294.4 64 305.1l0 56.7c0 10.9 10.7 18.6 21.1 15.2L256 320l0 80-57.6 43.2c-4 3-6.4 7.8-6.4 12.8l0 42c0 7.8 6.3 14 14 14c1.3 0 2.6-.2 3.9-.5L320 480l110.1 31.5c1.3 .4 2.6 .5 3.9 .5c7.8 0 14-6.3 14-14l0-42.7c0-2.8-.7-5.6-2.1-8L161.3 223.1z";
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
    exports.faPlaneUpSlash = exports.definition;
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

// src/_entries/faPlaneUpSlash.js
var import_faPlaneUpSlash = __toESM(require_faPlaneUpSlash());
var export_faPlaneUpSlash = import_faPlaneUpSlash.definition;
export {
  export_faPlaneUpSlash as faPlaneUpSlash
};

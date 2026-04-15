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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquareDribbble.js
var require_faSquareDribbble = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquareDribbble.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-dribbble";
    var width = 448;
    var height = 512;
    var aliases = ["dribbble-square"];
    var unicode = "f397";
    var svgPathData = "M165.9 132.5c-38.3 18-66.8 53.3-75.7 95.7 6.1 .1 62.4 .3 126.4-16.7-22.7-40.2-47.1-74.1-50.7-79zm26.1-9.1c3.8 5.1 28.6 38.9 51 80 48.6-18.3 69.1-45.9 71.6-49.4-33.6-29.8-79.3-41.1-122.6-30.6zM277.4 382c-2-12-10-53.8-29.2-103.6-55.1 18.8-93.8 56.4-108.1 85.6 40.5 31.6 93.3 36.7 137.3 18zM227.8 232.6C159.6 253 93.4 252.2 87.4 252l0 4.2c0 35.1 13.3 67.1 35.1 91.4 22.2-37.9 67.1-77.9 116.5-91.8-3.4-7.8-7.2-15.5-11.1-23.2l-.1 0zm72.5 136.9c30.7-20.7 52.5-53.6 58.6-91.6-4.6-1.5-42.3-12.7-85.1-5.8 17.9 49.1 25.1 89.1 26.5 97.4zm-34.8-119c45.5-5.7 90.7 3.4 95.2 4.4-.3-32.3-11.8-61.9-30.9-85.1-2.9 3.9-25.8 33.2-76.3 53.9 4.8 9.8 8.3 17.8 12 26.8zM384 32L64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64zM224 96a160 160 0 1 1 0 320 160 160 0 1 1 0-320z";
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
    exports.faSquareDribbble = exports.definition;
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

// src/_entries/faSquareDribbble.js
var import_faSquareDribbble = __toESM(require_faSquareDribbble());
var export_faSquareDribbble = import_faSquareDribbble.definition;
export {
  export_faSquareDribbble as faSquareDribbble
};

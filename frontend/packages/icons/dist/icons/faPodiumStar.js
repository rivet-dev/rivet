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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPodiumStar.js
var require_faPodiumStar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPodiumStar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "podium-star";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f758";
    var svgPathData = "M168 72c-22.1 0-40 17.9-40 40l0 32 296 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L24 192c-13.3 0-24-10.7-24-24s10.7-24 24-24l56 0 0-32c0-48.6 39.4-88 88-88l14.4 0C190.7 9.7 206.2 0 224 0l64 0c26.5 0 48 21.5 48 48s-21.5 48-48 48l-64 0c-17.8 0-33.3-9.7-41.6-24L168 72zM73.7 455.1L48 224l352 0L374.3 455.1c-3.6 32.4-31 56.9-63.6 56.9l-173.4 0c-32.6 0-60-24.5-63.6-56.9zM231.3 276.8c-2.9-6.4-11.7-6.4-14.5 0l-20.2 45.5c-1.2 2.6-3.6 4.4-6.3 4.8l-47.9 5.9c-6.8 .8-9.5 9.5-4.5 14.3l35.4 34c2 2 3 4.9 2.4 7.7l-9.4 49.1c-1.3 6.9 5.8 12.3 11.8 8.9l42.1-24.4c2.4-1.4 5.4-1.4 7.8 0L270 446.9c6 3.5 13.1-1.9 11.8-8.9L272.3 389c-.5-2.8 .4-5.8 2.4-7.7l35.4-34c5-4.8 2.3-13.5-4.5-14.3L257.8 327c-2.8-.3-5.2-2.1-6.3-4.8l-20.2-45.5z";
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
    exports.faPodiumStar = exports.definition;
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

// src/_entries/faPodiumStar.js
var import_faPodiumStar = __toESM(require_faPodiumStar());
var export_faPodiumStar = import_faPodiumStar.definition;
export {
  export_faPodiumStar as faPodiumStar
};

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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faNestingDolls.js
var require_faNestingDolls = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faNestingDolls.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "nesting-dolls";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3ba";
    var svgPathData = "M192 0c61.9 0 112 50.1 112 112l0 112L80 224l0-112C80 50.1 130.1 0 192 0zm-8.7 64.9c-11.7 15.1-28.7 26-48.2 29.7c-4 .8-7.2 4.2-6.8 8.2c1.5 14 7.6 27.7 18.4 38.5c25 25 65.5 25 90.5 0c10.8-10.8 16.9-24.4 18.4-38.5c.4-4.1-2.7-7.5-6.8-8.2c-19.5-3.7-36.5-14.6-48.2-29.7c-4.1-5.3-13.2-5.3-17.3 0zM635.5 474.1c5.2 7.3 5.9 16.9 1.8 24.9s-12.4 13-21.3 13l-304 0c-9 0-17.2-5-21.3-13c-4.1-8-3.4-17.6 1.8-24.9c0 0 0 0 0 0l.2-.3c.2-.3 .6-.8 1-1.5c.9-1.4 2.3-3.5 4-6.3c3.4-5.6 7.9-13.8 12.5-23.8c9.2-20.3 17.8-46.8 17.8-74.1l0-8c0-75.1 60.9-136 136-136s136 60.9 136 136l0 8c0 27.3 8.6 53.7 17.8 74.1c4.6 10 9.1 18.2 12.5 23.8c1.7 2.8 3.1 5 4 6.3c.5 .7 .8 1.2 1 1.5l.2 .3s0 0 0 0s0 0 0 0zM455.3 320.9C440.7 339.8 417.8 352 392 352c-4.3 0-8 3.3-7.6 7.6c1.7 17.9 9.4 35.3 23.1 49c31.2 31.2 81.9 31.2 113.1 0c13.7-13.7 21.4-31.1 23.1-49c.4-4.3-3.3-7.6-7.6-7.6c-25.8 0-48.7-12.2-63.3-31.1c-4.1-5.3-13.2-5.3-17.3 0zM296 360l0 8c0 21-6.8 42.7-15 60.8c-4 8.7-7.9 15.8-10.8 20.6c-1.4 2.4-2.6 4.1-3.2 5.2c-.3 .5-.6 .9-.7 1c0 0 0 0 0 0c0 0 0 0 0 .1c0 0 0 0 0 0s0 0 0 0c-11.7 16.6-13.5 38.1-4.9 56.3L74.5 512c-17 0-33.4-6.7-42.7-20.9C17.9 469.7 0 432.8 0 384c0-44.4 20.2-84.4 37.9-119.4c2.7-5.3 8.1-8.6 14-8.6l280.2 0C309.5 284.6 296 320.7 296 360z";
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
    exports.faNestingDolls = exports.definition;
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

// src/_entries/faNestingDolls.js
var import_faNestingDolls = __toESM(require_faNestingDolls());
var export_faNestingDolls = import_faNestingDolls.definition;
export {
  export_faNestingDolls as faNestingDolls
};

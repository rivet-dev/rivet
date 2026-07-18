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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCheeseSwiss.js
var require_faCheeseSwiss = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCheeseSwiss.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cheese-swiss";
    var width = 512;
    var height = 512;
    var aliases = [129472];
    var unicode = "f7f0";
    var svgPathData = "M451 93c37.7 37.7 61 89.7 61 147.2l0 15.8-15.8 0L448 256 96 256l-80 0L0 256c0-5.4 .7-10.7 2.1-15.8c3.8-13.7 12.4-25.8 24.5-33.9L96 160l23.2-15.4c7.8-5.2 18.4-2.3 25.5 3.8C153.1 155.6 164 160 176 160c26.5 0 48-21.5 48-48c0-5.2-.8-10.2-2.3-14.9c-2.9-9-1.6-19.9 6.3-25.1l47-31.3c8.6-5.7 18.6-8.7 28.9-8.7c56 0 106.8 22.1 144.2 58c1 1 2 2 3 2.9zM432 288l16 0 48 0 16 0 0 16 0 112c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 304l0-16 16 0 80 0 208 0c8.8 0 15.7 7.4 18.6 15.7C329.1 322.5 347 336 368 336s38.9-13.5 45.4-32.3c2.9-8.4 9.8-15.7 18.6-15.7zM224 384a48 48 0 1 0 -96 0 48 48 0 1 0 96 0z";
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
    exports.faCheeseSwiss = exports.definition;
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

// src/_entries/faCheeseSwiss.js
var import_faCheeseSwiss = __toESM(require_faCheeseSwiss());
var export_faCheeseSwiss = import_faCheeseSwiss.definition;
export {
  export_faCheeseSwiss as faCheeseSwiss
};

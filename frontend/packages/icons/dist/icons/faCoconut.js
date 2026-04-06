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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoconut.js
var require_faCoconut = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoconut.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coconut";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2f6";
    var svgPathData = "M141.2 141.2c-43.7 43.7-73 91.8-85.8 133.3c-13.2 42.7-7.4 71.9 7.5 86.7s44 20.6 86.7 7.5c41.5-12.8 89.6-42.1 133.3-85.8s73-91.8 85.8-133.3c13.2-42.7 7.4-71.9-7.5-86.7s-44-20.6-86.7-7.5C233 68.1 184.9 97.5 141.2 141.2zM260.4 9.5C309.6-5.7 361.5-4.8 395.2 28.8l40.9 40.9C484.7 118.4 512 184.3 512 252.9C512 396 396 512 253 512c-68.7 0-134.6-27.3-183.2-75.9l-41-41C-4.8 361.5-5.7 309.6 9.5 260.3c15.6-50.4 49.7-105 97.8-153.1S210 25 260.4 9.5zm63.5 88.1c25 25-5.4 95.9-67.9 158.4s-133.4 92.9-158.4 67.9s5.4-95.9 67.9-158.4S298.9 72.6 323.9 97.6zm95.4 107.1c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l32 32c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-32-32zm-56 136c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l32 32c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-32-32zm-136 56c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6l32 32c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6l-32-32z";
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
    exports.faCoconut = exports.definition;
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

// src/_entries/faCoconut.js
var import_faCoconut = __toESM(require_faCoconut());
var export_faCoconut = import_faCoconut.definition;
export {
  export_faCoconut as faCoconut
};

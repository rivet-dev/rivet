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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPizza.js
var require_faPizza = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPizza.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pizza";
    var width = 576;
    var height = 512;
    var aliases = [127829];
    var unicode = "f817";
    var svgPathData = "M385.2 419c10.4-8.2 25.5-8.4 34.9 1s9.4 24.7-.8 33.1C375 489.9 318.1 512 256 512C114.6 512 0 397.4 0 256S114.6 0 256 0c62.1 0 119 22.1 163.3 58.8c10.2 8.5 10.1 23.8 .8 33.1s-24.5 9.2-34.9 1C349.7 64.8 304.8 48 256 48C141.1 48 48 141.1 48 256s93.1 208 208 208c48.8 0 93.7-16.8 129.2-45zm-16-49.9c6.2 6.2 6.3 16.5-.5 22.1c-69.1 57.7-172.2 54.1-237.1-10.8c-68.7-68.7-68.7-180.2 0-248.9C196.5 66.6 299.5 63 368.6 120.8c6.8 5.7 6.7 15.9 .5 22.1L267.3 244.7c-6.2 6.2-6.2 16.4 0 22.6L369.1 369.1zM192 264a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm24-88a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm72 200a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM443.4 156.6c6.7-6.7 17.7-6.2 23 1.7c39.5 59 39.5 136.5 0 195.5c-5.2 7.8-16.3 8.3-23 1.7l-88.1-88.1c-6.2-6.2-6.2-16.4 0-22.6l24.2-24.2c3.6 2.2 7.9 3.5 12.5 3.5c13.3 0 24-10.7 24-24c0-4.6-1.3-8.9-3.5-12.5l30.9-30.9zM472 280a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm22.4 126.4c-8.9-8.9-9.4-23.1-2.3-33.5C514.8 339.6 528 299.3 528 256s-13.2-83.6-35.9-116.9c-7.1-10.4-6.6-24.6 2.3-33.5c9.9-9.9 26.2-9.4 34.3 2C558.5 149.5 576 200.7 576 256s-17.5 106.5-47.4 148.4c-8.1 11.4-24.4 11.9-34.3 2z";
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
    exports.faPizza = exports.definition;
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

// src/_entries/faPizza.js
var import_faPizza = __toESM(require_faPizza());
var export_faPizza = import_faPizza.definition;
export {
  export_faPizza as faPizza
};
